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

    let routes = collect_routes(&app, &["page"]);
    assert_eq!(routes.len(), 2);
    assert_eq!(routes[0].pattern, "/");
    assert_eq!(routes[1].pattern, "/users");

    // Test sorting tiebreaker
    fs::write(app.join("users/layout.tsx"), "").unwrap();
    let routes = collect_routes(&app, &["page", "layout"]);
    assert_eq!(routes.len(), 3);
}

#[test]
fn test_collect_routes_missing_root() {
    let routes = collect_routes(Path::new("missing"), &["page"]);
    assert!(routes.is_empty());
}

#[test]
fn test_collect_routes_empty() {
    let dir = tempfile::tempdir().unwrap();
    let routes = collect_routes(dir.path(), &["page"]);
    assert!(routes.is_empty());
}
