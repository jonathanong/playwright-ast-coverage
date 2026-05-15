use crate::analysis::app_collect::collect_app_selectors;
use crate::analysis::output::{build_related_report, print_edges_text, print_related_text};
use crate::analysis::pipeline::{analyze, run};
use crate::cli::{Cli, Command};
use crate::config::Settings;
use crate::selectors;
use crate::test_support::fixture_path;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

#[test]
fn analyze_discovers_tests_and_builds_reports() {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.extend(["tests", "fixtures", "covered"]);
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
    assert_eq!(run(cli_edges).unwrap(), ExitCode::SUCCESS);

    let mut cli_related = cli.clone();
    cli_related.command = Command::Related {
        files: vec![PathBuf::from("app/page.tsx")],
    };
    assert_eq!(run(cli_related).unwrap(), ExitCode::SUCCESS);

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
}

#[test]
fn analyze_surfaces_parser_errors() {
    let root = fixture_path(&["main", "invalid-test-source"]);
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

    let root = fixture_path(&["main", "invalid-selector-source"]);
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
