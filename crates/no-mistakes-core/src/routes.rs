use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const PAGE_EXTS: &[&str] = &["tsx", "ts", "jsx", "js"];

pub struct Route {
    pub file: PathBuf,
    pub pattern: String,
}

pub fn collect_routes(frontend_root: &Path, stems: &[&str]) -> Result<Vec<Route>> {
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
        if !stems.contains(&stem) || !PAGE_EXTS.contains(&ext) {
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
    use std::fs;

    #[test]
    fn test_path_to_route_pattern() {
        assert_eq!(path_to_route_pattern(Path::new("page.tsx")), "/");
        assert_eq!(path_to_route_pattern(Path::new("users/page.tsx")), "/users");
        assert_eq!(
            path_to_route_pattern(Path::new("(auth)/login/page.tsx")),
            "/login"
        );
        assert_eq!(
            path_to_route_pattern(Path::new("@sidebar/settings/page.tsx")),
            "/settings"
        );
        assert_eq!(
            path_to_route_pattern(Path::new("blog/[slug]/page.tsx")),
            "/blog/:slug"
        );
        assert_eq!(
            path_to_route_pattern(Path::new("shop/[[...rest]]/page.tsx")),
            "/shop/**"
        );
        assert_eq!(
            path_to_route_pattern(Path::new("docs/[...all]/page.tsx")),
            "/docs/*"
        );
        assert_eq!(
            path_to_route_pattern(Path::new("(group)/@parallel/page.tsx")),
            "/"
        );

        // Test non-normal components
        assert_eq!(path_to_route_pattern(Path::new("a/../b/page.tsx")), "/a/b");
    }

    #[test]
    fn test_collect_routes() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("app");
        fs::create_dir(&app).unwrap();
        fs::write(app.join("page.tsx"), "").unwrap();
        fs::create_dir(app.join("users")).unwrap();
        fs::write(app.join("users/page.tsx"), "").unwrap();
        fs::write(app.join("not-a-page.ts"), "").unwrap();

        let routes = collect_routes(&app, &["page"]).unwrap();
        assert_eq!(routes.len(), 2);
        assert_eq!(routes[0].pattern, "/");
        assert_eq!(routes[1].pattern, "/users");

        // Test sorting tiebreaker
        fs::write(app.join("users/layout.tsx"), "").unwrap();
        let routes = collect_routes(&app, &["page", "layout"]).unwrap();
        assert_eq!(routes.len(), 3);
    }

    #[test]
    fn test_collect_routes_missing_root() {
        let routes = collect_routes(Path::new("missing"), &["page"]).unwrap();
        assert!(routes.is_empty());
    }

    #[test]
    fn test_collect_routes_empty() {
        let dir = tempfile::tempdir().unwrap();
        let routes = collect_routes(dir.path(), &["page"]).unwrap();
        assert!(routes.is_empty());
    }
}
