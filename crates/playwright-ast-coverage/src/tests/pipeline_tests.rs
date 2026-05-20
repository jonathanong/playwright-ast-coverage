use crate::analysis::app_collect::collect_app_selectors;
use crate::analysis::context::{DiscoveredTestFile, TestAnalysisContext, TestProjectContext};
use crate::analysis::output::{build_related_report, print_edges_text, print_related_text};
use crate::analysis::pipeline::{analyze, run};
use crate::analysis::test_file::analyze_test_file;
use crate::cli::{Cli, Command};
use crate::config::Settings;
use crate::selectors;
use crate::test_support::fixture_path;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

#[test]
fn analyze_discovers_tests_and_builds_reports() {
    let root = fixture_path(&["nextjs-coverage", "covered"]);
    let settings = Settings {
        frontend_root: "web/app".to_string(),
        playwright_configs: vec![],
        project: None,
        test_include: vec![],
        test_exclude: vec![],
        ignore_routes: vec![],
        navigation_helpers: vec![],
        selector_attributes: vec!["data-testid".to_string()],
        component_selector_attributes: BTreeMap::new(),
        html_ids: false,
        selector_roots: vec!["web/app".to_string()],
        selector_include: vec![],
        selector_exclude: vec![],
    };

    let analysis = analyze(&root, &settings).unwrap();
    assert!(!analysis.coverage.routes.is_empty());
    assert!(!analysis.edges.edges.is_empty());

    let run_root = root.join("web");
    let cli = Cli {
        root: run_root.clone(),
        config: None,
        playwright_config: vec![],
        project: None,
        json: false,
        assert_conditional_tests: false,
        allow_skipped_tests: false,
        assert_unique_test_ids: false,
        assert_unique_html_ids: false,
        assert_unique_selectors: false,
        command: Command::Check,
    };
    assert_eq!(run(cli.clone()).unwrap(), ExitCode::from(1));

    let mut cli_json = cli.clone();
    cli_json.json = true;
    assert_eq!(run(cli_json).unwrap(), ExitCode::from(1));

    let mut cli_edges = cli.clone();
    cli_edges.command = Command::Edges;
    assert_eq!(run(cli_edges.clone()).unwrap(), ExitCode::SUCCESS);

    let mut cli_edges_json = cli_edges;
    cli_edges_json.json = true;
    assert_eq!(run(cli_edges_json).unwrap(), ExitCode::SUCCESS);

    let mut cli_related = cli.clone();
    cli_related.command = Command::Related {
        files: vec![PathBuf::from("app/page.tsx")],
    };
    assert_eq!(run(cli_related.clone()).unwrap(), ExitCode::SUCCESS);

    let mut cli_related_json = cli_related;
    cli_related_json.json = true;
    assert_eq!(run(cli_related_json).unwrap(), ExitCode::SUCCESS);

    let mut cli_unique = cli.clone();
    cli_unique.assert_unique_selectors = true;
    cli_unique.assert_unique_html_ids = true;
    assert_eq!(run(cli_unique).unwrap(), ExitCode::from(1));

    print_edges_text(&analysis.edges);
    let related = build_related_report(
        &root,
        &analysis.edges.edges,
        &[PathBuf::from("web/app/page.tsx")],
    );
    print_related_text(&related);
    let _ = serde_json::to_string_pretty(&analysis).unwrap();

    let mut cli_tests = cli.clone();
    cli_tests.command = Command::Tests {
        files: vec![PathBuf::from("web/app/page.tsx")],
    };
    assert_eq!(run(cli_tests.clone()).unwrap(), ExitCode::SUCCESS);

    let mut cli_tests_json = cli_tests;
    cli_tests_json.json = true;
    assert_eq!(run(cli_tests_json).unwrap(), ExitCode::SUCCESS);
}

#[test]
fn analyze_surfaces_parser_errors() {
    let root = fixture_path(&["ast-snippets", "main", "invalid-test-source"]);
    let settings = Settings {
        frontend_root: "web/app".to_string(),
        playwright_configs: vec![],
        project: None,
        test_include: vec!["tests/**/*.spec.ts".to_string()],
        test_exclude: vec![],
        ignore_routes: vec![],
        navigation_helpers: vec![],
        selector_attributes: vec![],
        component_selector_attributes: BTreeMap::new(),
        html_ids: false,
        selector_roots: vec!["web/app".to_string()],
        selector_include: vec![],
        selector_exclude: vec![],
    };

    let err = analyze(&root, &settings).err().unwrap();
    assert!(err.to_string().contains("failed to parse"));

    let root = fixture_path(&["ast-snippets", "main", "invalid-selector-source"]);
    let settings = Settings {
        frontend_root: "web/app".to_string(),
        playwright_configs: vec![],
        project: None,
        test_include: vec![],
        test_exclude: vec![],
        ignore_routes: vec![],
        navigation_helpers: vec![],
        selector_attributes: vec!["data-testid".to_string()],
        component_selector_attributes: BTreeMap::new(),
        html_ids: false,
        selector_roots: vec!["web/app".to_string()],
        selector_include: vec![],
        selector_exclude: vec![],
    };
    let selector_regexes = selectors::compile_selector_regexes(
        &settings.selector_attributes,
        &settings.component_selector_attributes,
    );
    let err = collect_app_selectors(&root, &settings, &selector_regexes)
        .err()
        .unwrap();
    assert!(err.to_string().contains("failed to parse"));
}

#[test]
fn analyze_test_file_with_selector_targets_extracts_edges() {
    // Uses a fixture that has data-testid selectors so app_selector_targets is
    // non-empty, exercising the `else` branch in analyze_test_file.
    let root = fixture_path(&["nextjs-selectors", "selector-covered"]);
    let settings = Settings {
        frontend_root: "web/app".to_string(),
        playwright_configs: vec![],
        project: None,
        test_include: vec![],
        test_exclude: vec![],
        ignore_routes: vec![],
        navigation_helpers: vec![],
        selector_attributes: vec!["data-testid".to_string()],
        component_selector_attributes: BTreeMap::new(),
        html_ids: false,
        selector_roots: vec!["web/app".to_string()],
        selector_include: vec![],
        selector_exclude: vec![],
    };
    let analysis = analyze(&root, &settings).unwrap();
    let selector_edges: Vec<_> = analysis
        .edges
        .edges
        .iter()
        .filter(|e| matches!(e, crate::analysis::types::Edge::Selector { .. }))
        .collect();
    assert!(
        !selector_edges.is_empty(),
        "expected selector edges when app_selector_targets is non-empty"
    );
}

#[test]
fn analyze_test_file_returns_error_for_missing_file() {
    // Exercises the `?` error branch in analyze_test_file when the file doesn't exist.
    use crate::analysis::context::{RouteIndex, SelectorIndex};
    use crate::playwright_tests::TestPolicy;
    let root = fixture_path(&["nextjs-coverage", "covered"]);
    let test_file = DiscoveredTestFile {
        path: PathBuf::from("/nonexistent/test.spec.ts"),
        contexts: vec![TestProjectContext {
            base_url: None,
            test_id_attribute: "data-testid".to_string(),
        }],
    };
    let route_index = RouteIndex::default();
    let selector_index = SelectorIndex::default();
    let selector_regexes = selectors::compile_selector_regexes(&[], &BTreeMap::new());
    let context = TestAnalysisContext {
        root: &root,
        route_index: &route_index,
        app_selector_targets: &[],
        selector_index: &selector_index,
        navigation_helpers: &[],
        selector_regexes: &selector_regexes,
        test_policy: TestPolicy::default(),
    };
    let err = analyze_test_file(&test_file, &context);
    assert!(err.is_err(), "expected error for non-existent test file");
}

#[test]
fn analyze_test_file_returns_error_for_parse_failure() {
    use crate::analysis::context::{RouteIndex, SelectorIndex};
    use crate::playwright_tests::TestPolicy;

    let root = fixture_path(&["react-traits-components", "bad-file"]);
    let test_file = DiscoveredTestFile {
        path: root.join("app/components/Broken.tsx"),
        contexts: vec![TestProjectContext {
            base_url: None,
            test_id_attribute: "data-testid".to_string(),
        }],
    };
    let route_index = RouteIndex::default();
    let selector_index = SelectorIndex::default();
    let selector_regexes = selectors::compile_selector_regexes(&[], &BTreeMap::new());
    let context = TestAnalysisContext {
        root: &root,
        route_index: &route_index,
        app_selector_targets: &[],
        selector_index: &selector_index,
        navigation_helpers: &[],
        selector_regexes: &selector_regexes,
        test_policy: TestPolicy::default(),
    };

    let err = analyze_test_file(&test_file, &context)
        .err()
        .expect("expected parse failure");

    assert!(!err.to_string().is_empty());
}
