use super::*;

fn fixture(name: &str) -> PathBuf {
    crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis")
            .join(name),
    )
}

#[test]
fn report_marks_matching_dynamic_route_covered() {
    let root = fixture("playwright-coverage");
    let all_files = crate::codebase::ts_source::discover_files(&root, &[]);
    let report = collect_report_from_files(&root, None, &[], &all_files).unwrap();

    let user_route = report
        .routes
        .iter()
        .find(|route| route.route == "/users/:id")
        .expect("expected user route");

    assert!(user_route.covered);
    assert_eq!(
        user_route.tests,
        vec![RouteTestHit {
            file: "tests/e2e/routes.spec.ts".to_string(),
            url: "/users/42".to_string(),
        }]
    );
}

#[test]
fn report_marks_unmatched_route_uncovered() {
    let root = fixture("playwright-coverage");
    let all_files = crate::codebase::ts_source::discover_files(&root, &[]);
    let report = collect_report_from_files(&root, None, &[], &all_files).unwrap();

    let settings_route = report
        .routes
        .iter()
        .find(|route| route.route == "/settings")
        .expect("expected settings route");

    assert!(!settings_route.covered);
}

#[test]
fn report_uses_explicit_frontend_root() {
    let root = fixture("playwright-coverage-alt");
    let all_files = crate::codebase::ts_source::discover_files(&root, &[]);
    let frontend_root = root.join("apps/site/app");
    let test_globs = vec!["specs/**/*.ts".to_string()];
    let report =
        collect_report_from_files(&root, Some(&frontend_root), &test_globs, &all_files).unwrap();

    assert_eq!(report.summary.total, 1);
    assert_eq!(report.summary.covered, 1);
    assert_eq!(report.routes[0].route, "/dashboard");
}

#[test]
fn explicit_frontend_root_must_exist() {
    let root = fixture("playwright-coverage");
    let missing = root.join("web/ap");

    let err = resolve_frontend_root(Some(&missing), &root, None).unwrap_err();

    assert!(err.to_string().contains("does not exist"));
}

#[test]
fn configured_frontend_root_must_exist() {
    let root = fixture("playwright-coverage");
    let yaml = r#"
rules:
  route-consistency:
    frontendRoot: "web/ap"
"#;
    let config = crate::codebase::config::Config::from_yaml(yaml).unwrap();

    let err = resolve_frontend_root(None, &root, Some(&config)).unwrap_err();

    assert!(err.to_string().contains("does not exist"));
}

#[test]
fn malformed_route_consistency_options_fall_back_to_default_frontend_root() {
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(temp.path().join("web/app")).unwrap();
    let yaml = r#"
rules:
  route-consistency:
    frontendRoot: 123
"#;
    let config = crate::codebase::config::Config::from_yaml(yaml).unwrap();

    let frontend_root = resolve_frontend_root(None, temp.path(), Some(&config)).unwrap();

    assert_eq!(frontend_root, temp.path().join("web/app"));
}

#[test]
fn collect_report_from_files_surfaces_malformed_guardrails_config() {
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(temp.path().join("web/app")).unwrap();
    std::fs::write(temp.path().join(".guardrailsrc.yml"), "rules: [").unwrap();
    let all_files = crate::codebase::ts_source::discover_files(temp.path(), &[]);

    let err = collect_report_from_files(temp.path(), None, &[], &all_files).unwrap_err();

    assert!(format!("{err:#}").contains("loading guardrails config"));
}

#[test]
fn skip_file_patterns_match_normalized_relative_paths() {
    let root = PathBuf::from("/repo");
    let files = vec![
        root.join("web\\app\\generated\\page.tsx"),
        root.join("web/app/page.tsx"),
    ];

    let filtered = filter_skip_file_patterns(&root, files, &["^web/app/generated/".to_string()]);

    assert_eq!(filtered, vec![root.join("web/app/page.tsx")]);
}

#[test]
fn missing_route_consistency_config_uses_default_frontend_root() {
    let root = fixture("playwright-coverage");
    let config = crate::codebase::config::Config::default();

    let frontend_root = resolve_frontend_root(None, &root, Some(&config)).unwrap();

    assert_eq!(frontend_root, root.join("web/app"));
}
