use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const PAGE_EXTS: &[&str] = &["tsx", "ts", "jsx", "js"];

pub struct Route {
    pub file: PathBuf,
    pub pattern: String,
}

pub fn collect_routes(frontend_root: &Path, stems: &[&str]) -> Vec<Route> {
    if !frontend_root.exists() {
        return Vec::new();
    }

    let mut routes = Vec::new();
    for entry in WalkDir::new(frontend_root)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !stems.contains(&stem) || !PAGE_EXTS.contains(&ext) {
            continue;
        }

        let Ok(relative) = path.strip_prefix(frontend_root) else {
            continue;
        };
        routes.push(Route {
            file: path.to_path_buf(),
            pattern: path_to_route_pattern(relative),
        });
    }

    routes.sort_by(|a, b| a.pattern.cmp(&b.pattern).then_with(|| a.file.cmp(&b.file)));
    routes
}

pub fn path_to_route_pattern(relative: &Path) -> String {
    let dir: PathBuf = relative
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or_default();

    let mut segments = Vec::new();
    for component in dir.components() {
        let std::path::Component::Normal(segment) = component else {
            continue;
        };
        let segment = segment.to_str().unwrap_or("");

        if segment.starts_with('@') || (segment.starts_with('(') && segment.ends_with(')')) {
            continue;
        }

        if segment.starts_with("[[...") && segment.ends_with("]]") {
            segments.push("**".to_string());
            continue;
        }

        if segment.starts_with("[...") && segment.ends_with(']') {
            segments.push("*".to_string());
            continue;
        }

        if segment.starts_with('[') && segment.ends_with(']') {
            segments.push(format!(":{}", &segment[1..segment.len() - 1]));
            continue;
        }

        segments.push(segment.to_string());
    }

    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}

#[cfg(test)]
mod tests;
