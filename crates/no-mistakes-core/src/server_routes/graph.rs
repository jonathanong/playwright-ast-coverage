use crate::server_routes::extract::extract_file;
use crate::server_routes::model::{FileFacts, ProjectReport, RouteSite};
use crate::server_routes::mounts::{prefixes_for, resolve_mounts};
use crate::server_routes::normalize::{join_paths, normalize_route};
use crate::server_routes::source::{discover_source_files, relative_string};
use crate::server_routes::types::{Diagnostic, Edge, EdgeKind, ServerRoute, Severity, Summary};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use std::collections::HashMap;
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
    let mounts = resolve_mounts(facts);
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
            for route in expand_site(root, site, facts, &mounts) {
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
    let dynamic_routes = routes
        .iter()
        .filter(|route| route.route.contains('*'))
        .count();
    ProjectReport {
        summary: Summary {
            total_routes: routes.len(),
            total_files: facts.len(),
            dynamic_routes,
        },
        routes,
        edges,
        diagnostics,
    }
}

fn expand_site(
    root: &Path,
    site: &RouteSite,
    facts: &HashMap<PathBuf, FileFacts>,
    mounts: &[crate::server_routes::mounts::ResolvedMount],
) -> Vec<ServerRoute> {
    prefixes_for(site, facts, mounts)
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
