use crate::server_routes::extract::extract_file;
use crate::server_routes::model::{FileFacts, ProjectReport, RouteSite};
use crate::server_routes::normalize::{join_paths, normalize_route};
use crate::server_routes::source::{discover_source_files, relative_string};
use crate::server_routes::types::{Diagnostic, Edge, EdgeKind, ServerRoute, Severity, Summary};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RelatedDirection {
    Deps,
    Dependents,
    Both,
}

pub fn analyze_project(
    root: &Path,
    _tsconfig_path: Option<&Path>,
    filters: &[String],
) -> anyhow::Result<ProjectReport> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let filter = build_filter(filters)?;
    let files = discover_source_files(&root)
        .into_iter()
        .filter(|path| {
            filter
                .as_ref()
                .is_none_or(|f| f.is_match(path.strip_prefix(&root).unwrap_or(path)))
        })
        .collect::<Vec<_>>();
    let facts = files
        .iter()
        .filter_map(|path| extract_file(path).ok().map(|facts| (path.clone(), facts)))
        .collect::<HashMap<_, _>>();
    Ok(build_report(&root, &facts))
}

fn build_report(root: &Path, facts: &HashMap<PathBuf, FileFacts>) -> ProjectReport {
    let mut routes = Vec::new();
    let mut edges = Vec::new();
    let mut diagnostics = Vec::new();
    for (path, file_facts) in facts {
        diagnostics.extend(
            file_facts
                .diagnostics
                .iter()
                .map(|(line, message)| Diagnostic {
                    severity: Severity::Warning,
                    file: relative_string(root, path),
                    line: *line,
                    message: message.clone(),
                }),
        );
        for site in &file_facts.routes {
            for route in expand_site(root, site, file_facts) {
                edges.push(Edge {
                    from: route.file.clone(),
                    to: route.route.clone(),
                    kind: EdgeKind::ServerRoute,
                });
                routes.push(route);
            }
        }
    }
    routes.sort();
    routes.dedup();
    edges.sort();
    edges.dedup();
    diagnostics.sort();
    diagnostics.dedup();
    ProjectReport {
        summary: Summary {
            total_routes: routes.len(),
            total_files: facts.len(),
            dynamic_routes: diagnostics.len(),
        },
        routes,
        edges,
        diagnostics,
    }
}

fn expand_site(root: &Path, site: &RouteSite, facts: &FileFacts) -> Vec<ServerRoute> {
    let mut prefixes = Vec::new();
    if let Some(binding) = facts.bindings.get(&site.binding) {
        prefixes.extend(binding.prefixes.clone());
    }
    prefixes.extend(mount_prefixes(&site.binding, facts));
    if prefixes.is_empty() {
        prefixes.push(String::new());
    }
    prefixes
        .into_iter()
        .map(|prefix| {
            let raw_path = join_paths(&prefix, &site.raw_path);
            ServerRoute {
                file: relative_string(root, &site.file),
                line: site.line,
                method: site.method.clone(),
                route: normalize_route(&raw_path),
                raw_path,
                framework: site.framework,
            }
        })
        .collect()
}

fn mount_prefixes(binding: &str, facts: &FileFacts) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    collect_mount_prefixes(binding, facts, "", &mut seen, &mut out);
    out
}

fn collect_mount_prefixes(
    binding: &str,
    facts: &FileFacts,
    suffix: &str,
    seen: &mut HashSet<String>,
    out: &mut Vec<String>,
) {
    if !seen.insert(binding.to_string()) {
        return;
    }
    for mount in facts.mounts.iter().filter(|mount| mount.child == binding) {
        let prefix = join_paths(&mount.prefix, suffix);
        out.push(prefix.clone());
        if let Some(parent) = facts.bindings.get(&mount.parent) {
            out.extend(
                parent
                    .prefixes
                    .iter()
                    .map(|parent_prefix| join_paths(parent_prefix, &prefix)),
            );
        }
        collect_mount_prefixes(&mount.parent, facts, &prefix, seen, out);
    }
}

fn build_filter(filters: &[String]) -> anyhow::Result<Option<GlobSet>> {
    if filters.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for filter in filters {
        builder.add(GlobBuilder::new(filter).literal_separator(false).build()?);
    }
    Ok(Some(builder.build()?))
}
