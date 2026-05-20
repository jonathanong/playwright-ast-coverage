use super::*;
use crate::server_routes::types::Framework;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/ast-snippets/server-routes")
        .join(name)
}

#[test]
fn extract_file_covers_import_binding_route_and_mount_shapes() {
    let facts = extract_file(&fixture("extract-walk-all.ts")).unwrap();

    assert!(facts
        .imports
        .iter()
        .any(|import| import.imported == "Router" && import.local == "StringRouter"));
    assert_eq!(facts.exports["publicRouter"], "router");
    assert_eq!(facts.exports["default"], "defaultThing");
    assert_eq!(facts.bindings["api"].framework, Framework::Express);
    assert_eq!(facts.bindings["hono"].prefixes, vec!["/hono"]);
    assert_eq!(facts.bindings["koa"].prefixes, vec!["/koa"]);
    assert_eq!(facts.bindings["loose"].prefixes, vec!["/loose"]);

    let route_pairs: Vec<_> = facts
        .routes
        .iter()
        .map(|route| (route.method.as_str(), route.raw_path.as_str()))
        .collect();
    for expected in [
        ("get", "/direct"),
        ("delete", "/del"),
        ("get", "/"),
        ("get", "/array"),
        ("get", "/template-array"),
        ("get", "/spread-array"),
        ("get", "/named"),
        ("get", "/root"),
        ("get", "/on"),
        ("delete", "/on"),
        ("get", "/koa-no-prefix"),
        ("get", "/hono-no-prefix"),
        ("get", "/hono-plain"),
        ("get", "/matched"),
        ("get", "/child"),
        ("post", "/post"),
        ("put", "/put"),
        ("get", "/api-server"),
        ("get", "/heuristic"),
    ] {
        assert!(
            route_pairs.contains(&expected),
            "missing route {expected:?}"
        );
    }
    for skipped in [
        ("get", "/client-supertest-chain"),
        ("get", "/client-supertest-variable"),
        ("get", "/client-axios"),
        ("post", "/client-axios-create"),
        ("get", "/client-got"),
        ("put", "/client-ky"),
        ("get", "/client-superagent"),
        ("get", "/client-playwright"),
        ("get", "/client-axios-static-object"),
        ("get", "/client-node-http"),
    ] {
        assert!(
            !route_pairs.contains(&skipped),
            "client request should not be a route definition {skipped:?}"
        );
    }

    assert!(facts
        .mounts
        .iter()
        .any(|mount| mount.parent == "api" && mount.child == "router" && mount.prefix == "/"));
    assert!(facts.mounts.iter().any(|mount| {
        mount.parent == "api" && mount.child == "stringRouter" && mount.prefix == "/api-route"
    }));
}

#[test]
fn default_export_non_identifier_is_ignored() {
    let facts = extract_file(&fixture("default-function.ts")).unwrap();

    assert_eq!(facts.exports["default"], "default");
}
