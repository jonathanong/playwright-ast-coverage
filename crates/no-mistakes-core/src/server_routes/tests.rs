use super::*;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/server-ast-routes")
        .join(name)
}

#[test]
fn express_project_reports_route_edges() {
    let report = analyze_project(&fixture("express"), None, &[]).unwrap();
    assert!(report
        .routes
        .iter()
        .any(|route| route.route == "/api/v1/users/*" && route.method == "get"));
    assert!(report
        .edges
        .iter()
        .any(|edge| edge.from == "backend/api/users.ts" && edge.to == "/api/v1/users/*"));
}

#[test]
fn hono_project_reports_prefixed_routes() {
    let report = analyze_project(&fixture("hono"), None, &[]).unwrap();
    assert!(report
        .routes
        .iter()
        .any(|route| route.route == "/api/posts/*" && route.method == "get"));
}

#[test]
fn koa_router_named_routes_and_mounts_are_supported() {
    let report = analyze_project(&fixture("koa-router"), None, &[]).unwrap();
    assert!(report
        .routes
        .iter()
        .any(|route| route.route == "/api/users/*" && route.method == "delete"));
}

#[test]
fn related_crosses_route_edges() {
    let report = analyze_project(&fixture("express"), None, &[]).unwrap();
    let edges = related(
        &report,
        &["backend/api/users.ts".to_string()],
        RelatedDirection::Deps,
    );
    assert!(edges.iter().any(|edge| edge.to == "/api/v1/users/*"));
}

#[test]
fn filters_limit_discovered_sources() {
    let report = analyze_project(
        &fixture("express"),
        None,
        &["backend/api/users.ts".to_string()],
    )
    .unwrap();
    assert_eq!(report.summary.total_files, 1);
}

#[test]
fn mixed_framework_shapes_are_supported() {
    let report = analyze_project(&fixture("mixed"), None, &[]).unwrap();
    for expected in [
        "/array/*",
        "/array/*/edit",
        "/api-server/*",
        "/matched/*",
        "/v1/koa/*",
        "/child/hono-child/*",
        "/paren/*",
    ] {
        assert!(
            report.routes.iter().any(|route| route.route == expected),
            "missing {expected}"
        );
    }
}

#[test]
fn related_dependents_and_both_are_supported() {
    let report = analyze_project(&fixture("mixed"), None, &[]).unwrap();
    let dependents = related(
        &report,
        &["/api-server/*".to_string()],
        RelatedDirection::Dependents,
    );
    assert!(dependents
        .iter()
        .any(|edge| edge.to == "backend/api/routes.ts"));
    let both = related(
        &report,
        &["backend/api/routes.ts".to_string()],
        RelatedDirection::Both,
    );
    assert!(both.iter().any(|edge| edge.to == "/matched/*"));
}
