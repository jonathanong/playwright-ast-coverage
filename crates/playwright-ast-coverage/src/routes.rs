use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const PAGE_STEMS: &[&str] = &["page"];
const PAGE_EXTS: &[&str] = &["tsx", "ts", "jsx", "js"];

pub struct Route {
    pub file: PathBuf,
    pub pattern: String,
}

pub fn collect_routes(frontend_root: &Path) -> Result<Vec<Route>> {
    if !frontend_root.exists() {
        return Ok(Vec::new());
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
        if !PAGE_STEMS.contains(&stem) || !PAGE_EXTS.contains(&ext) {
            continue;
        }

        if let Ok(relative) = path.strip_prefix(frontend_root) {
            routes.push(Route {
                file: path.to_path_buf(),
                pattern: path_to_route_pattern(relative),
            });
        }
    }

    routes.sort_by(|a, b| a.pattern.cmp(&b.pattern).then_with(|| a.file.cmp(&b.file)));
    Ok(routes)
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
mod tests {
    use super::*;
    use crate::test_support::fixture_path;

    #[test]
    fn root_page_maps_to_slash() {
        let p = Path::new("page.tsx");
        assert_eq!(path_to_route_pattern(p), "/");
    }

    #[test]
    fn route_group_is_skipped() {
        let p = Path::new("(user)/user/[idOrUsername]/page.tsx");
        assert_eq!(path_to_route_pattern(p), "/user/:idOrUsername");
    }

    #[test]
    fn slug_dynamic_segment() {
        let p = Path::new("communities/[slug]/settings/page.tsx");
        assert_eq!(path_to_route_pattern(p), "/communities/:slug/settings");
    }

    #[test]
    fn catch_all_maps_to_wildcard() {
        let p = Path::new("[...rest]/page.tsx");
        assert_eq!(path_to_route_pattern(p), "/*");
    }

    #[test]
    fn static_nested_path() {
        let p = Path::new("communities/page.tsx");
        assert_eq!(path_to_route_pattern(p), "/communities");
    }

    #[test]
    fn collect_frontend_routes_finds_pages() {
        let routes: Vec<String> = collect_routes(&fixture_path(&["routes", "collect"]))
            .unwrap()
            .into_iter()
            .map(|route| route.pattern)
            .collect();
        assert!(routes.contains(&"/communities".to_string()));
        assert!(routes.contains(&"/communities/:slug".to_string()));
        assert!(routes.contains(&"/user/:id".to_string()));
    }

    #[test]
    fn collect_frontend_routes_sorts_duplicate_patterns_by_file() {
        let routes = collect_routes(&fixture_path(&["routes", "sort-duplicates"])).unwrap();
        assert_eq!(routes[0].pattern, "/same");
        assert!(routes[0].file <= routes[1].file);
    }

    #[test]
    fn collect_frontend_routes_missing_root_returns_empty() {
        let routes = collect_routes(&fixture_path(&["routes", "missing"])).unwrap();
        assert!(routes.is_empty());
    }

    #[test]
    fn absolute_path_components_are_ignored() {
        let p = Path::new("/communities/page.tsx");
        assert_eq!(path_to_route_pattern(p), "/communities");
    }
}
