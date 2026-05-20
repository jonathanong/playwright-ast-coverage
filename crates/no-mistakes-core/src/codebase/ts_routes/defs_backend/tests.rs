use super::*;
use std::path::PathBuf;

fn route_fixture_source(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/ast-snippets/ts-routes")
        .join(name);
    std::fs::read_to_string(path).expect("route fixture source must be readable")
}

#[test]
fn extracts_simple_get_route() {
    let source = "app.route('/api/v1/users').get(handler);";
    let routes = extract_backend_routes(source, "app");
    assert_eq!(routes, vec![("/api/v1/users".to_string(), 1)]);
}

#[test]
fn extracts_route_with_param() {
    let source = "app.route('/api/v1/users/:id').put(handler);";
    let routes = extract_backend_routes(source, "app");
    assert_eq!(routes, vec![("/api/v1/users/:id".to_string(), 1)]);
}

#[test]
fn extracts_direct_verb_route() {
    let source = "app.get('/api/v1/users/:id', handler);";
    let routes = extract_backend_routes(source, "app");
    assert_eq!(routes, vec![("/api/v1/users/:id".to_string(), 1)]);
}

#[test]
fn extracts_direct_verb_route_nested_in_call_argument() {
    let source = "wrap(app.get('/api/v1/users/:id', handler));";
    let routes = extract_backend_routes(source, "app");
    assert_eq!(routes, vec![("/api/v1/users/:id".to_string(), 1)]);
}

#[test]
fn ignores_direct_verb_route_on_shadowed_register_object_parameter() {
    let source = "function helper(app) { app.get('/tmp', handler); }";
    let routes = extract_backend_routes(source, "app");
    assert!(routes.is_empty());
}

#[test]
fn ignores_chain_route_on_shadowed_register_object_variable() {
    let source = "function helper() { const app = fake; app.route('/tmp').get(handler); }";
    let routes = extract_backend_routes(source, "app");
    assert!(routes.is_empty());
}

#[test]
fn ignores_direct_verb_route_after_register_object_var_in_loop() {
    let source = "function helper() { for (var app = fake; ready; ready = false) {} app.get('/tmp', handler); }";
    let routes = extract_backend_routes(source, "app");
    assert!(routes.is_empty());
}

#[test]
fn ignores_direct_verb_route_after_register_object_var_in_for_of() {
    let source = "function helper() { for (var app of apps) {} app.get('/tmp', handler); }";
    let routes = extract_backend_routes(source, "app");
    assert!(routes.is_empty());
}

#[test]
fn ignores_direct_verb_route_after_register_object_var_in_for_in() {
    let source = "function helper() { for (var app in apps) {} app.get('/tmp', handler); }";
    let routes = extract_backend_routes(source, "app");
    assert!(routes.is_empty());
}

#[test]
fn ignores_direct_verb_route_on_class_shadowed_register_object() {
    let source = "function helper() { class app {} app.get('/tmp', handler); }";
    let routes = extract_backend_routes(source, "app");
    assert!(routes.is_empty());
}

#[test]
fn ignores_direct_verb_route_on_function_name_register_object() {
    let source = "function app() { app.get('/tmp', handler); }";
    let routes = extract_backend_routes(source, "app");
    assert!(routes.is_empty());
}

#[test]
fn extracts_static_template_route() {
    let source = "app.route(`/api/v1/users/:id`).get(handler);";
    let routes = extract_backend_routes(source, "app");
    assert_eq!(routes, vec![("/api/v1/users/:id".to_string(), 1)]);
}

#[test]
fn ignores_interpolated_template_route() {
    let source = "app.route(`/api/v1/users/${id}`).get(handler);";
    let routes = extract_backend_routes(source, "app");
    assert!(routes.is_empty());
}

#[test]
fn chained_methods_same_route() {
    let source = "app.route('/path').get(h1).post(h2);";
    let routes = extract_backend_routes(source, "app");
    assert_eq!(routes.len(), 2);
    assert!(routes.iter().all(|(p, _)| p == "/path"));
}

#[test]
fn non_route_call_is_ignored() {
    let source = "doSomething(); foo.bar();";
    let routes = extract_backend_routes(source, "app");
    assert!(routes.is_empty());
}

#[test]
fn all_http_verbs_are_recognized() {
    for verb in &["get", "post", "put", "patch", "delete", "head", "options"] {
        let source = format!("app.route('/test').{verb}(handler);");
        let routes = extract_backend_routes(&source, "app");
        assert_eq!(routes.len(), 1);
    }
}

#[test]
fn empty_source_produces_no_routes() {
    let routes = extract_backend_routes("", "app");
    assert!(routes.is_empty());
}

#[test]
fn fixture_routes_backend_extracts_nested_router_pattern() {
    // fixtures/routes-backend mirrors the generic backend route pattern:
    //   import app from '../../app.mts'
    //   app.route('/api/v1/users/:idOrSlug').get(...).patch(...).delete(...)
    let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis/routes-backend/backend/api/v1/users/user.mts");
    let source = std::fs::read_to_string(&fixture).expect("fixture file should exist");
    let routes = extract_backend_routes(&source, "app");
    assert_eq!(routes.len(), 3, "expected 3 routes (get, patch, delete)");
    let patterns: Vec<&str> = routes.iter().map(|(p, _)| p.as_str()).collect();
    assert!(
        patterns.iter().all(|p| *p == "/api/v1/users/:idOrSlug"),
        "all routes should be /api/v1/users/:idOrSlug, got: {patterns:?}"
    );
}

#[test]
fn fixture_backend_walker_covers_statement_shadowing_and_route_shapes() {
    let source = route_fixture_source("backend-walk-all.ts");
    let routes = extract_backend_routes(&source, "app");
    let patterns: Vec<&str> = routes
        .iter()
        .map(|(pattern, _line)| pattern.as_str())
        .collect();

    assert!(patterns.contains(&"/chain"));
    assert!(patterns.contains(&"/direct"));
    assert!(patterns.contains(&"/wrapped"));
    assert!(patterns.contains(&"/block"));
    assert!(patterns.contains(&"/export-var"));
    assert!(patterns.contains(&"/export-function"));
    assert!(!patterns.iter().any(|pattern| pattern.contains("ignored")));
}

#[test]
fn collect_backend_routes_helpers_filter_files_and_directories() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ast-snippets");
    let source = root.join("ts-routes/backend-walk-all.ts");
    let unmatched = root.join("server-routes/default-function.ts");
    let outside = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let mut builder = globset::GlobSetBuilder::new();
    builder.add(globset::Glob::new("ts-routes/**/*.ts").unwrap());
    let globset = builder.build().unwrap();

    let from_files = collect_backend_routes_from_files(
        &root,
        &[root.clone(), outside, unmatched, source.clone()],
        "app",
        &globset,
    );
    assert!(from_files.iter().any(|(_, route)| route == "/chain"));

    let from_dir = collect_backend_routes_in_dir(&root, "app", &globset);
    assert!(from_dir.iter().any(|(_, route)| route == "/chain"));
    assert!(!globset.is_match("ts-routes/backend-walk-all.tsx"));
}
