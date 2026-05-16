use crate::fetch::cache::Cache;
use crate::fetch::file_analysis::analyze_file;
use crate::fetch::import_routes::is_route_handler_file;
use crate::fetch::types::FetchOccurrence;
use crate::routes::Route;
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn collect_route_fetches(
    route: &Route,
    frontend_root: &Path,
    root: &Path,
    cache: &mut Cache,
) -> Result<Vec<FetchOccurrence>> {
    let route_is_page = route.file.file_stem().and_then(|s| s.to_str()) == Some("page");
    let route_is_route_handler = is_route_handler_file(&route.file);

    let mut visited = HashSet::new();
    let mut fetches = Vec::new();

    let _route_is_client = analyze_file(
        &route.file,
        root,
        &mut visited,
        &mut fetches,
        cache,
        false,
        route_is_route_handler,
    )?;

    if route_is_page {
        collect_page_layout_fetches(
            route,
            frontend_root,
            root,
            cache,
            &mut visited,
            &mut fetches,
        )?;
    }

    fetches.sort();
    Ok(fetches)
}

fn collect_page_layout_fetches(
    route: &Route,
    frontend_root: &Path,
    root: &Path,
    cache: &mut Cache,
    visited: &mut HashSet<(PathBuf, bool, bool)>,
    fetches: &mut Vec<FetchOccurrence>,
) -> Result<()> {
    let route_is_route_handler = is_route_handler_file(&route.file);
    let mut current = route.file.parent();
    while let Some(parent) = current {
        if !parent.starts_with(frontend_root) {
            break;
        }

        for stem in ["layout", "loading", "error", "not-found", "template"] {
            for ext in ["tsx", "ts", "jsx", "js"] {
                let layout_file = parent.join(format!("{stem}.{ext}"));
                if layout_file.exists() {
                    analyze_file(
                        &layout_file,
                        root,
                        visited,
                        fetches,
                        cache,
                        false,
                        route_is_route_handler,
                    )?;
                }
            }
        }
        current = parent.parent();
    }
    Ok(())
}
