use crate::analyze::resolve::is_client_route_file;
use crate::analyze::routes::{
    collect_layout_chain_files, is_route_handler_file, route_reaches_target,
};
use crate::pipeline::target::{resolve_target_file, route_matches_target};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn fixture(category: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(category)
        .join(name)
}

#[test]
fn test_route_matches_target() {
    assert!(route_matches_target("/users", "users"));
    assert!(!route_matches_target("/users-team", "users"));
    assert!(route_matches_target("/users/team", "users/"));
    assert!(!route_matches_target("/users-team/page", "users/"));
    assert!(route_matches_target("/users", "/users"));
    assert!(!route_matches_target("/users/team", "/users"));
    assert!(route_matches_target("/users/team", "/users/"));
    assert!(route_matches_target("/", "/"));
    assert!(!route_matches_target("/users", "/"));
}

#[test]
fn test_route_matches_target_rejects_empty_input() {
    assert!(!route_matches_target("/users", ""));
    assert!(!route_matches_target("/users", "   "));
}

#[test]
fn test_is_route_handler_file_variants() {
    assert!(is_route_handler_file(Path::new("route.ts")));
    assert!(is_route_handler_file(Path::new("route")));
    assert!(!is_route_handler_file(Path::new("page.tsx")));
    assert!(!is_route_handler_file(Path::new("not-route.txt")));
}

#[test]
fn test_collect_layout_chain_files_includes_parent_chain() {
    let app = fixture("next-to-fetch-routes", "layout-chain").join("app");
    let page = app.join("dashboard/page.tsx");

    let chain = collect_layout_chain_files(&page, &app);
    assert_eq!(chain.len(), 4);
    assert!(chain.contains(&app.join("template.tsx")));
    assert!(chain.contains(&app.join("dashboard/layout.tsx")));
    assert!(chain.contains(&app.join("layout.tsx")));
    assert!(chain.contains(&app.join("dashboard/template.tsx")));
}

#[test]
fn test_is_client_route_file_missing_file() {
    assert!(!is_client_route_file(Path::new("does-not-exist.ts")).unwrap());
}

#[test]
fn test_is_client_route_file_with_use_client_directive() {
    let file = fixture("next-to-fetch-routes", "use-client").join("client.ts");
    assert!(is_client_route_file(&file).unwrap());
}

#[test]
fn test_is_client_route_file_without_use_client_directive() {
    let file = fixture("next-to-fetch-routes", "no-use-client").join("server.ts");
    assert!(!is_client_route_file(&file).unwrap());
}

#[test]
fn test_resolve_target_file_errors() {
    let root = fixture("next-to-fetch-routes", "simple-file");

    let empty = resolve_target_file(&root, "   ");
    assert!(empty.is_err());
    let err = empty.unwrap_err();
    assert!(err.to_string().contains("target path cannot be empty"));

    let file = root.join("route.ts");
    let absolute = file.canonicalize().unwrap();
    let resolved = resolve_target_file(&root, absolute.to_str().unwrap()).unwrap();
    assert_eq!(resolved, absolute);

    // layout-chain has an "app" subdirectory — passing a directory should fail
    let layout_root = fixture("next-to-fetch-routes", "layout-chain");
    let not_file = resolve_target_file(&layout_root, "app");
    assert!(not_file.is_err());
    let err = not_file.unwrap_err();
    assert!(err.to_string().contains("target path is not a file"));
}

#[test]
fn test_route_reaches_target_short_circuit() {
    let route = fixture("next-to-fetch-routes", "simple-file").join("route.ts");
    let target = fixture("next-to-fetch-routes", "route-reaches").join("target.ts");

    let mut cache = HashMap::new();
    let mut visited = std::collections::HashSet::new();
    let route_abs = route.canonicalize().unwrap();
    let target_abs = target.canonicalize().unwrap();
    assert!(!route_reaches_target(&route, &target_abs, &mut visited, &mut cache).unwrap());

    visited.insert(route_abs);
    let matched_direct =
        route_reaches_target(&route, &target_abs, &mut visited, &mut cache).unwrap();
    assert!(!matched_direct);
}

#[test]
fn test_route_reaches_target_matches_direct() {
    let route = fixture("next-to-fetch-routes", "simple-file").join("route.ts");
    let mut cache = HashMap::new();
    let mut visited = std::collections::HashSet::new();
    let route_abs = route.canonicalize().unwrap();
    let reached = route_reaches_target(&route, &route_abs, &mut visited, &mut cache).unwrap();
    assert!(reached);
}

#[test]
fn test_route_reaches_target_via_import() {
    let base = fixture("next-to-fetch-routes", "route-reaches");
    let route = base.join("route.ts");
    let target = base.join("target.ts");

    let mut cache = HashMap::new();
    let mut visited = std::collections::HashSet::new();
    assert!(route_reaches_target(
        &route,
        &target.canonicalize().unwrap(),
        &mut visited,
        &mut cache
    )
    .unwrap());
}

#[test]
fn test_route_reaches_target_nonexistent_source_returns_error() {
    // When path doesn't exist, canonicalize() fails — exercises the `?` error branch.
    let base = fixture("next-to-fetch-routes", "route-reaches");
    let nonexistent = base.join("ghost.ts");
    let target = base.join("target.ts");

    let mut cache = HashMap::new();
    let mut visited = std::collections::HashSet::new();
    let result = route_reaches_target(&nonexistent, &target, &mut visited, &mut cache);
    assert!(
        result.is_err(),
        "non-existent source should return an error"
    );
}

#[test]
fn test_route_reaches_target_nonexistent_target_uses_fallback() {
    // When target doesn't exist, canonicalize() fails and unwrap_or_else fallback is used.
    let route = fixture("next-to-fetch-routes", "simple-file").join("route.ts");
    let nonexistent_target = route.parent().unwrap().join("does-not-exist.ts");
    let mut cache = HashMap::new();
    let mut visited = std::collections::HashSet::new();
    // route != nonexistent_target, so returns false.
    let result =
        route_reaches_target(&route, &nonexistent_target, &mut visited, &mut cache).unwrap();
    assert!(!result);
}

#[test]
#[cfg(unix)]
fn test_is_client_route_file_unreadable_returns_error() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempdir().unwrap();
    let file = dir.path().join("unreadable.ts");
    let fixture_src = fixture("next-to-fetch-routes", "no-use-client").join("server.ts");
    fs::copy(&fixture_src, &file).unwrap();
    let mut perms = file.metadata().unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&file, perms).unwrap();

    let result = is_client_route_file(&file);
    assert!(result.is_err(), "unreadable file should return an error");
}

#[test]
fn test_route_reaches_target_collect_imports_parse_error() {
    // A file with a parse error causes collect_imports to fail,
    // exercising the `?` error branch in routes.rs.
    let route = fixture("next-to-fetch-routes", "route-parse-error").join("route.ts");
    let target = fixture("next-to-fetch-routes", "route-reaches").join("target.ts");

    let mut cache = HashMap::new();
    let mut visited = std::collections::HashSet::new();
    let result = route_reaches_target(
        &route,
        &target.canonicalize().unwrap(),
        &mut visited,
        &mut cache,
    );
    assert!(result.is_err(), "parse error in route should propagate");
}

#[test]
fn test_route_reaches_target_with_unmatched_import_chain() {
    let base = fixture("next-to-fetch-routes", "unmatched-chain");
    let route = base.join("route.ts");
    // target.ts is in a different fixture dir, so the chain route→middle→leaf never reaches it.
    let target = fixture("next-to-fetch-routes", "route-reaches").join("target.ts");

    let mut cache = HashMap::new();
    let mut visited = std::collections::HashSet::new();
    assert!(!route_reaches_target(
        &route,
        &target.canonicalize().unwrap(),
        &mut visited,
        &mut cache
    )
    .unwrap());
}
