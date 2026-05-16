use crate::fetch::imports::collect_imports;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub fn is_route_handler_file(path: &Path) -> bool {
    path.file_stem().and_then(|stem| stem.to_str()) == Some("route")
}

pub fn route_reaches_target(
    path: &Path,
    target: &Path,
    visited: &mut HashSet<PathBuf>,
    import_cache: &mut HashMap<PathBuf, Vec<PathBuf>>,
) -> Result<bool> {
    let abs_path = path.canonicalize()?;
    let abs_target = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());
    if abs_path == abs_target {
        return Ok(true);
    }
    if visited.contains(&abs_path) {
        return Ok(false);
    }
    visited.insert(abs_path.clone());

    for import in collect_imports(&abs_path, import_cache)? {
        if route_reaches_target(&import, &abs_target, visited, import_cache)? {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn collect_layout_chain_files(route_file: &Path, frontend_root: &Path) -> Vec<PathBuf> {
    let mut layout_files = Vec::new();
    let mut current = route_file.parent();
    while let Some(parent) = current {
        if !parent.starts_with(frontend_root) {
            break;
        }

        for stem in ["layout", "loading", "error", "not-found", "template"] {
            for ext in ["tsx", "ts", "jsx", "js"] {
                let layout_file = parent.join(format!("{stem}.{ext}"));
                if layout_file.exists() {
                    layout_files.push(layout_file);
                }
            }
        }

        current = parent.parent();
    }

    layout_files
}
