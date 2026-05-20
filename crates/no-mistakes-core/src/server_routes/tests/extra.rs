use crate::server_routes::analyze_project;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/server-ast-routes")
        .join(name)
}

#[test]
fn tsconfig_paths_resolve_mounted_routers() {
    let root = fixture("tsconfig-paths");
    let report = analyze_project(&root, Some(&root.join("tsconfig.json")), &[]).unwrap();
    let relative =
        analyze_project(&root, Some(std::path::Path::new("tsconfig.json")), &[]).unwrap();

    assert!(report
        .routes
        .iter()
        .any(|route| route.route == "/api/users/*" && route.file == "src/routes/users.ts"));
    assert!(report
        .routes
        .iter()
        .any(|route| route.route == "/cjs/ping" && route.file == "src/routes/common.cts"));
    assert_eq!(relative.routes, report.routes);
}

#[test]
fn implicit_invalid_tsconfig_falls_back_but_explicit_errors() {
    let root = fixture("invalid-tsconfig");

    let report = analyze_project(&root, None, &[]).unwrap();
    assert!(report.routes.iter().any(|route| route.route == "/health"));

    let err = analyze_project(&root, Some(&root.join("tsconfig.json")), &[]).unwrap_err();
    assert!(format!("{err:#}").contains("loading tsconfig"));
}

#[test]
fn malformed_v2_config_falls_back_to_unconfigured_scan() {
    let report = analyze_project(&fixture("malformed-config"), None, &[]).unwrap();

    assert!(report
        .routes
        .iter()
        .any(|route| route.route == "/api/users/*" && route.method == "get"));
}

#[test]
fn invalid_project_route_globs_fall_back_to_unconfigured_scan() {
    let report = analyze_project(&fixture("invalid-route-glob"), None, &[]).unwrap();

    assert!(report
        .routes
        .iter()
        .any(|route| route.route == "/api/users/*" && route.method == "get"));
}
