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
fn exit_status_codes_match_process_exit_contract() {
    assert_eq!(ExitStatus::Covered.code(), 0);
    assert_eq!(ExitStatus::Uncovered.code(), 1);
}

#[test]
fn resolve_format_prefers_json_flag_explicit_format_then_tty_default() {
    assert_eq!(
        resolve_format(true, Some(Format::Human), true),
        Format::Json
    );
    assert_eq!(resolve_format(false, Some(Format::Md), true), Format::Md);
    assert_eq!(resolve_format(false, None, true), Format::Human);
    assert_eq!(resolve_format(false, None, false), Format::Json);
}

#[test]
fn run_returns_uncovered_for_fixture_with_missing_route() {
    let root = fixture("playwright-coverage");
    let status = run(CoverageArgs {
        root: Some(root),
        frontend_root: None,
        test_globs: Vec::new(),
        format: Some(Format::Json),
        json: false,
        timings: true,
    })
    .unwrap();

    assert_eq!(status, ExitStatus::Uncovered);
}

#[test]
fn run_returns_covered_with_explicit_frontend_root_and_json_shorthand() {
    let root = fixture("playwright-coverage-alt");
    let status = run(CoverageArgs {
        frontend_root: Some(PathBuf::from("apps/site/app")),
        test_globs: vec!["specs/**/*.ts".to_string()],
        root: Some(root),
        format: None,
        json: true,
        timings: false,
    })
    .unwrap();

    assert_eq!(status, ExitStatus::Covered);
}

#[test]
fn run_requires_routes_under_frontend_root() {
    let root = fixture("playwright-coverage-invalid-config");
    let err = run(CoverageArgs {
        frontend_root: Some(PathBuf::from("web/app")),
        test_globs: vec!["tests/e2e/**/*.ts".to_string()],
        root: Some(root),
        format: Some(Format::Human),
        json: false,
        timings: false,
    })
    .unwrap_err();

    assert!(format!("{err:#}").contains("no Next.js routes discovered"));
}

#[test]
fn run_surfaces_invalid_test_glob_from_report_collection() {
    let root = fixture("playwright-coverage");
    let err = run(CoverageArgs {
        frontend_root: Some(PathBuf::from("web/app")),
        test_globs: vec!["[".to_string()],
        root: Some(root),
        format: Some(Format::Json),
        json: false,
        timings: false,
    })
    .unwrap_err();

    assert!(
        format!("{err:#}").contains("invalid glob `[`"),
        "unexpected error: {err:#}"
    );
}

#[test]
fn run_surfaces_config_load_errors_without_explicit_frontend_root() {
    let root = fixture("playwright-coverage-invalid-config");
    let err = run(CoverageArgs {
        root: Some(root),
        frontend_root: None,
        test_globs: Vec::new(),
        format: Some(Format::Json),
        json: false,
        timings: false,
    })
    .unwrap_err();

    assert!(format!("{err:#}").contains("loading guardrails config"));
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
fn no_config_uses_default_frontend_root() {
    let root = fixture("playwright-coverage");

    let frontend_root = resolve_frontend_root(None, &root, None).unwrap();

    assert_eq!(frontend_root, root.join("web/app"));
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
fn skip_file_patterns_ignore_invalid_regexes_and_keep_external_paths() {
    let root = PathBuf::from("/repo");
    let external = PathBuf::from("/outside/page.tsx");
    let file = root.join("web/app/page.tsx");
    let files = vec![external.clone(), file.clone()];

    let filtered = filter_skip_file_patterns(&root, files, &["[".to_string()]);

    assert_eq!(filtered, vec![external, file]);
}

#[test]
fn skip_file_patterns_keep_external_paths_with_valid_patterns() {
    let root = PathBuf::from("/repo");
    let external = PathBuf::from("/outside/page.tsx");
    let generated = root.join("web/app/generated/page.tsx");
    let regular = root.join("web/app/page.tsx");
    let files = vec![external.clone(), generated, regular.clone()];

    let filtered = filter_skip_file_patterns(&root, files, &["^web/app/generated/".to_string()]);

    assert_eq!(filtered, vec![external, regular]);
}

#[test]
fn route_coverage_sort_uses_file_as_tiebreaker() {
    let mut routes = [
        RouteCoverage {
            route: "/same".to_string(),
            file: "b/page.tsx".to_string(),
            covered: false,
            tests: Vec::new(),
        },
        RouteCoverage {
            route: "/same".to_string(),
            file: "a/page.tsx".to_string(),
            covered: false,
            tests: Vec::new(),
        },
    ];

    routes.sort_by(compare_route_coverage);

    assert_eq!(routes[0].file, "a/page.tsx");
}

#[test]
fn missing_route_consistency_config_uses_default_frontend_root() {
    let root = fixture("playwright-coverage");
    let config = crate::codebase::config::Config::default();

    let frontend_root = resolve_frontend_root(None, &root, Some(&config)).unwrap();

    assert_eq!(frontend_root, root.join("web/app"));
}

#[test]
fn configured_frontend_root_uses_valid_route_options() {
    let root = fixture("codebase-intel");
    let config = crate::codebase::config::load_config(&root).unwrap();

    let frontend_root = resolve_frontend_root(None, &root, Some(&config)).unwrap();

    assert_eq!(frontend_root, root.join("packages/web/app"));
}

fn sample_report(uncovered: bool) -> CoverageReport {
    CoverageReport {
        summary: CoverageSummary {
            total: 2,
            covered: if uncovered { 1 } else { 2 },
            uncovered: if uncovered { 1 } else { 0 },
            coverage_percent: if uncovered { 50.0 } else { 100.0 },
        },
        routes: vec![
            RouteCoverage {
                route: "/covered".to_string(),
                file: "web/app/covered/page.tsx".to_string(),
                covered: true,
                tests: vec![RouteTestHit {
                    file: "tests/e2e/routes.spec.ts".to_string(),
                    url: "/covered".to_string(),
                }],
            },
            RouteCoverage {
                route: "/missing".to_string(),
                file: "web/app/missing/page.tsx".to_string(),
                covered: !uncovered,
                tests: Vec::new(),
            },
        ],
    }
}

#[test]
fn report_writers_cover_all_formats_for_uncovered_routes() {
    let report = sample_report(true);

    for format in [
        Format::Json,
        Format::Yml,
        Format::Paths,
        Format::Human,
        Format::Md,
    ] {
        let mut out = Vec::new();
        write_report(&report, format, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("missing") || text.contains("uncovered"));
    }
}

#[test]
fn human_and_markdown_reports_cover_all_routes_covered_branch() {
    let report = sample_report(false);

    let mut human = Vec::new();
    write_report(&report, Format::Human, &mut human).unwrap();
    assert!(String::from_utf8(human)
        .unwrap()
        .contains("All routes are covered."));

    let mut markdown = Vec::new();
    write_report(&report, Format::Md, &mut markdown).unwrap();
    assert!(String::from_utf8(markdown)
        .unwrap()
        .contains("_All routes are covered._"));
}

#[test]
fn path_and_glob_helpers_cover_relative_absolute_default_and_invalid_glob() {
    let cwd = PathBuf::from("/repo/current");
    assert_eq!(
        resolve_root(Some(Path::new("/abs/root")), &cwd),
        PathBuf::from("/abs/root")
    );
    assert_eq!(
        resolve_root(Some(Path::new("rel/root")), &cwd),
        cwd.join("rel/root")
    );
    assert_eq!(resolve_root(None, &cwd), cwd);

    assert_eq!(
        relative_string(Path::new("/repo"), Path::new("/repo/a/b.ts")),
        "a/b.ts"
    );
    assert_eq!(
        relative_string(Path::new("/repo"), Path::new("/other/b.ts")),
        "/other/b.ts"
    );

    assert_eq!(
        test_globs_or_default(&["custom/**/*.ts".to_string()]),
        vec!["custom/**/*.ts"]
    );
    assert!(test_globs_or_default(&[])
        .iter()
        .any(|glob| glob.contains("spec")));

    let err = build_globset(&["[".to_string()]).unwrap_err();
    assert!(format!("{err:#}").contains("invalid glob"));
}

#[test]
fn collect_playwright_visits_filters_sorts_deduplicates_and_skips_unreadable_files() {
    let root = fixture("playwright-coverage");
    let spec = root.join("tests/e2e/routes.spec.ts");
    let unreadable = root.join("tests/e2e/missing.spec.ts");
    let page = root.join("web/app/users/[id]/page.tsx");
    let all_files = vec![page, spec.clone(), spec, unreadable];
    let visits = collect_playwright_visits(
        root.as_path(),
        &["tests/e2e/**/*.ts".to_string()],
        &all_files,
    )
    .unwrap();

    assert_eq!(
        visits
            .iter()
            .map(|visit| (
                relative_string(root.as_path(), &visit.file),
                visit.url.as_str()
            ))
            .collect::<Vec<_>>(),
        vec![
            ("tests/e2e/routes.spec.ts".to_string(), "/"),
            ("tests/e2e/routes.spec.ts".to_string(), "/users/42"),
        ]
    );
}
