use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const PAGE_STEMS: &[&str] = &["page"];
const PAGE_EXTS: &[&str] = &["tsx", "ts", "jsx", "js"];

/// Walk `frontend_root` (absolute path) and return `(file_path, route_pattern)` pairs
/// for all `page.{tsx,ts,jsx,js}` files.
pub fn collect_frontend_routes_with_files(frontend_root: &Path) -> Vec<(PathBuf, String)> {
    let mut routes = Vec::new();

    for entry in WalkDir::new(frontend_root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        if !PAGE_STEMS.contains(&stem) || !PAGE_EXTS.contains(&ext) {
            continue;
        }

        if let Ok(relative) = path.strip_prefix(frontend_root) {
            routes.push((path.to_path_buf(), path_to_route_pattern(relative)));
        }
    }

    routes.sort_by(|a, b| a.1.cmp(&b.1));
    routes
}

/// Collect frontend routes from an already-discovered file list.
pub fn collect_frontend_routes_from_files(
    frontend_root: &Path,
    files: &[PathBuf],
) -> Vec<(PathBuf, String)> {
    let mut routes = Vec::new();

    for path in files.iter().filter(|path| path.starts_with(frontend_root)) {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        if !PAGE_STEMS.contains(&stem) || !PAGE_EXTS.contains(&ext) {
            continue;
        }

        if let Ok(relative) = path.strip_prefix(frontend_root) {
            routes.push((path.clone(), path_to_route_pattern(relative)));
        }
    }

    routes.sort_by(|a, b| a.1.cmp(&b.1));
    routes
}

/// Walk `frontend_root` and return all route patterns derived from page files.
pub fn collect_frontend_routes(frontend_root: &Path) -> Vec<String> {
    collect_frontend_routes_with_files(frontend_root)
        .into_iter()
        .map(|(_, p)| p)
        .collect()
}

/// Translate a relative path (relative to `frontend_root`) to a route pattern.
/// E.g. "(user)/user/[idOrUsername]/page.tsx" → "/user/:idOrUsername"
pub fn path_to_route_pattern(relative: &Path) -> String {
    let dir: PathBuf = relative
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_default();

    let mut segments: Vec<String> = Vec::new();

    for component in dir.components() {
        use std::path::Component;
        let seg = match component {
            Component::Normal(s) => s.to_str().unwrap_or(""),
            _ => continue,
        };

        let seg = strip_intercepting_prefix(seg);
        if should_skip_segment(seg) {
            continue;
        }

        if seg.starts_with("[[...") && seg.ends_with("]]") {
            segments.push("**".to_string());
            continue;
        }

        if seg.starts_with("[...") && seg.ends_with(']') {
            segments.push("*".to_string());
            continue;
        }

        if seg.starts_with('[') && seg.ends_with(']') {
            let param = &seg[1..seg.len() - 1];
            segments.push(format!(":{}", param));
            continue;
        }

        segments.push(seg.to_string());
    }

    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}

fn should_skip_segment(seg: &str) -> bool {
    (seg.starts_with('(') && seg.ends_with(')')) || seg.starts_with('@') || seg.starts_with('_')
}

fn strip_intercepting_prefix(seg: &str) -> &str {
    for prefix in ["(..)(..)", "(...)", "(..)", "(.)"] {
        if let Some(rest) = seg.strip_prefix(prefix) {
            return rest;
        }
    }
    seg
}

#[cfg(test)]
mod tests;
