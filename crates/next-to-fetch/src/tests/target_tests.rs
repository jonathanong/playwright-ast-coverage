use crate::analyze::resolve::is_client_route_file;
use crate::analyze::routes::{
    collect_layout_chain_files, is_route_handler_file, route_reaches_target,
};
use crate::pipeline::target::{resolve_target_file, route_matches_target};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

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
    let dir = tempdir().unwrap();
    let app = dir.path().join("app");
    fs::create_dir_all(app.join("dashboard")).unwrap();

    fs::write(app.join("layout.tsx"), "export {}").unwrap();
    fs::write(app.join("template.tsx"), "export {}").unwrap();
    fs::write(app.join("dashboard/layout.tsx"), "export {}").unwrap();
    fs::write(app.join("dashboard/template.tsx"), "export {}").unwrap();
    let page = app.join("dashboard/page.tsx");
    fs::write(&page, "export {}").unwrap();

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
    let dir = tempdir().unwrap();
    let file = dir.path().join("client.ts");
    fs::write(&file, "'use client';\nexport {};").unwrap();

    assert!(is_client_route_file(&file).unwrap());
}

#[test]
fn test_is_client_route_file_without_use_client_directive() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("server.ts");
    fs::write(&file, "export {};").unwrap();

    assert!(!is_client_route_file(&file).unwrap());
}

#[test]
fn test_resolve_target_file_errors() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("absolute.ts");
    fs::write(&file, "").unwrap();

    let empty = resolve_target_file(dir.path(), "   ");
    assert!(empty.is_err());
    let err = empty.unwrap_err();
    assert!(err.to_string().contains("target path cannot be empty"));

    let absolute = file.canonicalize().unwrap();
    let resolved = resolve_target_file(dir.path(), absolute.to_str().unwrap()).unwrap();
    assert_eq!(resolved, absolute);

    let dir_target = dir.path().join("dir");
    fs::create_dir(&dir_target).unwrap();
    let not_file = resolve_target_file(dir.path(), "dir");
    assert!(not_file.is_err());
    let err = not_file.unwrap_err();
    assert!(err.to_string().contains("target path is not a file"));
}

#[test]
fn test_route_reaches_target_short_circuit() {
    let dir = tempdir().unwrap();
    let route = dir.path().join("route.ts");
    let target = dir.path().join("target.ts");
    fs::write(&route, "").unwrap();
    fs::write(&target, "").unwrap();

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
    let dir = tempdir().unwrap();
    let route = dir.path().join("route.ts");
    fs::write(&route, "").unwrap();

    let mut cache = HashMap::new();
    let mut visited = std::collections::HashSet::new();
    let route_abs = route.canonicalize().unwrap();
    let reached = route_reaches_target(&route, &route_abs, &mut visited, &mut cache).unwrap();
    assert!(reached);
}

#[test]
fn test_route_reaches_target_via_import() {
    let dir = tempdir().unwrap();
    let route = dir.path().join("route.ts");
    let middle = dir.path().join("middle.ts");
    let target = dir.path().join("target.ts");
    fs::write(&route, "import { helper } from './middle';").unwrap();
    fs::write(&middle, "import { target } from './target';").unwrap();
    fs::write(&target, "").unwrap();

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
    // When path doesn't exist, canonicalize() fails — exercises the `?` error branch (line 16).
    let dir = tempdir().unwrap();
    let nonexistent = dir.path().join("ghost.ts");
    let target = dir.path().join("target.ts");
    fs::write(&target, "").unwrap();

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
    // When target doesn't exist, canonicalize() fails and unwrap_or_else fallback is used
    // (exercises line 19 col 36 in routes.rs).
    let dir = tempdir().unwrap();
    let route = dir.path().join("route.ts");
    fs::write(&route, "").unwrap();

    let nonexistent_target = dir.path().join("does-not-exist.ts");
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
    fs::write(&file, "export {};").unwrap();
    let mut perms = file.metadata().unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&file, perms).unwrap();

    let result = is_client_route_file(&file);
    assert!(result.is_err(), "unreadable file should return an error");
}

#[test]
fn test_route_reaches_target_collect_imports_parse_error() {
    // A file with a parse error causes collect_imports to fail,
    // exercising the `?` error branch at line 29 of routes.rs.
    let dir = tempdir().unwrap();
    let route = dir.path().join("route.ts");
    let target = dir.path().join("target.ts");
    // Write an unparseable file as the route (unclosed call fails oxc parser).
    fs::write(&route, "await page.goto(").unwrap();
    fs::write(&target, "").unwrap();

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
    let dir = tempdir().unwrap();
    let route = dir.path().join("route.ts");
    let middle = dir.path().join("middle.ts");
    let target = dir.path().join("target.ts");
    let leaf = dir.path().join("leaf.ts");
    fs::write(&route, "import { helper } from './middle';").unwrap();
    fs::write(&middle, "import { helper2 } from './leaf';").unwrap();
    fs::write(&leaf, "").unwrap();
    fs::write(&target, "").unwrap();

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
