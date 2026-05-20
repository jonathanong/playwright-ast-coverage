use crate::codebase::ts_resolver::{find_tsconfig, load_tsconfig, ImportResolver, TsConfig};
use crate::config::v2::{load_v2_config, ConfigView};
use crate::server_routes::extract::extract_file;
use crate::server_routes::model::{FileFacts, ProjectReport, RouteSite};
use crate::server_routes::mounts::{prefixes_for, resolve_mounts_with_resolver};
use crate::server_routes::normalize::{join_paths, normalize_route};
use crate::server_routes::source::{discover_source_files, relative_string};
use crate::server_routes::types::{Diagnostic, Edge, EdgeKind, ServerRoute, Severity, Summary};
use anyhow::Context;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use rayon::prelude::*;
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
    tsconfig_path: Option<&Path>,
    filters: &[String],
) -> anyhow::Result<ProjectReport> {
    let root = root.canonicalize().unwrap_or(root.to_path_buf());
    let tsconfig = resolve_tsconfig(&root, tsconfig_path)?;
    let v2_config = load_v2_config(&root, None).ok();
    let config_route_filter = v2_config
        .as_ref()
        .and_then(|config| build_filter(&ConfigView::new(config).server_route_globs()).ok())
        .flatten();
    let test_filter = v2_config
        .as_ref()
        .map(|config| crate::codebase::test_filter::TestFileFilter::new(&root, config));
    let extra_skip = v2_config
        .as_ref()
        .map(|config| config.filesystem.skip_directories.as_slice())
        .unwrap_or(&[]);
    let filter = build_filter(filters)?;
    let mut files = Vec::new();
    for path in discover_source_files(&root, extra_skip) {
        let rel = path.strip_prefix(&root).unwrap_or(&path);
        let matches_config = config_route_filter
            .as_ref()
            .map(|filter| filter.is_match(rel))
            .unwrap_or(true);
        let matches_cli = filter
            .as_ref()
            .map(|filter| filter.is_match(rel))
            .unwrap_or(true);
        let is_test = test_filter
            .as_ref()
            .is_some_and(|filter| filter.is_match(&root, &path));
        if matches_config && matches_cli && !is_test {
            files.push(path);
        }
    }

    let facts = collect_file_facts(&files);
    Ok(build_report(&root, &facts, &tsconfig))
}

pub(crate) fn route_defs_from_files(root: &Path, files: &[PathBuf]) -> Vec<(PathBuf, String)> {
    let root = root.canonicalize().unwrap_or(root.to_path_buf());
    let tsconfig =
        resolve_tsconfig(&root, None).expect("implicit tsconfig resolution should not fail");
    let facts = collect_file_facts(files);
    build_report(&root, &facts, &tsconfig)
        .routes
        .into_iter()
        .map(|route| (root.join(route.file), route.route))
        .collect()
}

fn collect_file_facts(files: &[PathBuf]) -> HashMap<PathBuf, FileFacts> {
    files
        .par_iter()
        .filter_map(|path| {
            extract_file(path)
                .ok()
                .map(|file_facts| (path.clone(), file_facts))
        })
        .collect()
}

pub(super) fn build_report(
    root: &Path,
    facts: &HashMap<PathBuf, FileFacts>,
    tsconfig: &TsConfig,
) -> ProjectReport {
    let mut routes = Vec::new();
    let mut edges = Vec::new();
    let mut diagnostics = Vec::new();
    let visible = facts.keys().cloned().collect::<HashSet<_>>();
    let resolver = ImportResolver::new(tsconfig).with_visible(&visible);
    let mounts = resolve_mounts_with_resolver(facts, &resolver);
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

fn resolve_tsconfig(root: &Path, explicit: Option<&Path>) -> anyhow::Result<TsConfig> {
    let explicit_path = explicit.is_some();
    let path = match explicit {
        Some(path) if path.is_absolute() => Some(path.to_path_buf()),
        Some(path) => Some(root.join(path)),
        None => find_tsconfig(root),
    };
    match path {
        Some(path) if explicit_path => {
            load_tsconfig(&path).context(format!("loading tsconfig {}", path.display()))
        }
        Some(path) => Ok(load_tsconfig(&path).unwrap_or_else(|_| empty_tsconfig(root))),
        None => Ok(empty_tsconfig(root)),
    }
}

fn empty_tsconfig(root: &Path) -> TsConfig {
    TsConfig {
        dir: root.to_path_buf(),
        paths_dir: root.to_path_buf(),
        paths: Vec::new(),
        base_url: None,
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
