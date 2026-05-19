use crate::analysis::discover::{build_project_discovery, discover_test_files};
use crate::analysis::output::{build_related_report, print_related_text};
use crate::analysis::pipeline::run;
use crate::analysis::types::Edge;
use crate::cli::{Cli, Command};
use crate::config::Settings;
use crate::playwright_config;
use crate::test_support::fixture_path;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[test]
fn run_check_errors_on_empty_app() {
    let root = fixture_path(&["scan-config", "missing-default"]);
    let cli = Cli {
        root,
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
    let err = run(cli).expect_err("empty app should error");
    assert!(err.to_string().contains("no Next.js page routes found"));
}

#[test]
fn run_errors_on_invalid_root() {
    let cli = Cli {
        root: PathBuf::from("/non-existent-path-12345/non-existent-child"),
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
    assert!(run(cli).is_err());
}

#[test]
fn run_errors_on_empty_routes() {
    let root = fixture_path(&["ast-snippets", "main", "empty-app"]);
    let cli = Cli {
        root,
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
    assert!(run(cli).is_err());
}

#[test]
fn run_errors_on_missing_playwright_config() {
    let root = fixture_path(&["nextjs-coverage", "covered"]);
    let cli = Cli {
        root: root.clone(),
        config: None,
        playwright_config: vec![root.join("missing.config.ts")],
        project: None,
        json: false,
        assert_conditional_tests: false,
        allow_skipped_tests: false,
        assert_unique_test_ids: false,
        assert_unique_html_ids: false,
        assert_unique_selectors: false,
        command: Command::Check,
    };
    assert!(run(cli).is_err());
}

#[test]
fn discover_test_files_walks_shared_project_test_dir_once() {
    let root = fixture_path(&["ast-snippets", "main", "analyze-basic"]);
    let settings = Settings {
        frontend_root: "web/app".to_string(),
        playwright_configs: vec![],
        project: None,
        test_include: vec![],
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
    let playwright = playwright_config::PlaywrightConfig {
        name: None,
        projects: vec![
            playwright_config::TestProject {
                config_dir: root.clone(),
                test_dir: "tests".to_string(),
                test_match: vec!["**/*.spec.ts".to_string()],
                test_ignore: vec![],
                base_url: Some("http://localhost:3000".to_string()),
                test_id_attribute: "data-testid".to_string(),
            },
            playwright_config::TestProject {
                config_dir: root.clone(),
                test_dir: "tests".to_string(),
                test_match: vec!["**/*.spec.ts".to_string()],
                test_ignore: vec![],
                base_url: Some("http://localhost:4000".to_string()),
                test_id_attribute: "data-pw".to_string(),
            },
        ],
    };

    let files = discover_test_files(&root, &settings, &playwright).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].contexts.len(), 2);
}

#[test]
fn discover_test_files_applies_yaml_exclude_before_project_matching() {
    let root = fixture_path(&["ast-snippets", "main", "analyze-basic"]);
    let settings = Settings {
        frontend_root: "web/app".to_string(),
        playwright_configs: vec![],
        project: None,
        test_include: vec![],
        test_exclude: vec!["tests/**".to_string()],
        ignore_routes: vec![],
        navigation_helpers: vec![],
        selector_attributes: vec![],
        component_selector_attributes: BTreeMap::new(),
        html_ids: false,
        selector_roots: vec!["web/app".to_string()],
        selector_include: vec![],
        selector_exclude: vec![],
    };
    let playwright = playwright_config::PlaywrightConfig {
        name: None,
        projects: vec![playwright_config::TestProject {
            config_dir: root.clone(),
            test_dir: "tests".to_string(),
            test_match: vec!["**/*.spec.ts".to_string()],
            test_ignore: vec![],
            base_url: None,
            test_id_attribute: "data-testid".to_string(),
        }],
    };

    let files = discover_test_files(&root, &settings, &playwright).unwrap();
    assert!(files.is_empty());
}

#[test]
fn discover_test_files_rejects_invalid_configured_globs() {
    let root = fixture_path(&["ast-snippets", "main", "analyze-basic"]);
    let base_settings = Settings {
        frontend_root: "web/app".to_string(),
        playwright_configs: vec![],
        project: None,
        test_include: vec![],
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
    let playwright = playwright_config::PlaywrightConfig {
        name: None,
        projects: vec![playwright_config::TestProject {
            config_dir: root.clone(),
            test_dir: "tests".to_string(),
            test_match: vec!["**/*.spec.ts".to_string()],
            test_ignore: vec![],
            base_url: None,
            test_id_attribute: "data-testid".to_string(),
        }],
    };

    let mut invalid_include = base_settings.clone();
    invalid_include.test_include = vec!["[".to_string()];
    assert!(discover_test_files(&root, &invalid_include, &playwright).is_err());

    let mut invalid_include_exclude = base_settings.clone();
    invalid_include_exclude.test_include = vec!["tests/**/*.spec.ts".to_string()];
    invalid_include_exclude.test_exclude = vec!["[".to_string()];
    assert!(discover_test_files(&root, &invalid_include_exclude, &playwright).is_err());

    let mut invalid_yaml_exclude = base_settings;
    invalid_yaml_exclude.test_exclude = vec!["[".to_string()];
    assert!(discover_test_files(&root, &invalid_yaml_exclude, &playwright).is_err());

    let invalid_project = playwright_config::PlaywrightConfig {
        name: None,
        projects: vec![playwright_config::TestProject {
            config_dir: root.clone(),
            test_dir: "tests".to_string(),
            test_match: vec!["[".to_string()],
            test_ignore: vec![],
            base_url: None,
            test_id_attribute: "data-testid".to_string(),
        }],
    };
    let settings = Settings {
        frontend_root: "web/app".to_string(),
        playwright_configs: vec![],
        project: None,
        test_include: vec![],
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
    assert!(discover_test_files(&root, &settings, &invalid_project).is_err());
}

#[test]
fn build_project_discovery_rejects_invalid_project_globs() {
    let root = fixture_path(&["ast-snippets", "main", "analyze-basic"]);
    let invalid_match = playwright_config::PlaywrightConfig {
        name: None,
        projects: vec![playwright_config::TestProject {
            config_dir: root.clone(),
            test_dir: "tests".to_string(),
            test_match: vec!["[".to_string()],
            test_ignore: vec![],
            base_url: None,
            test_id_attribute: "data-testid".to_string(),
        }],
    };
    assert!(build_project_discovery(&root, &invalid_match).is_err());

    let invalid_ignore = playwright_config::PlaywrightConfig {
        name: None,
        projects: vec![playwright_config::TestProject {
            config_dir: root.clone(),
            test_dir: "tests".to_string(),
            test_match: vec!["**/*.spec.ts".to_string()],
            test_ignore: vec!["[".to_string()],
            base_url: None,
            test_id_attribute: "data-testid".to_string(),
        }],
    };
    assert!(build_project_discovery(&root, &invalid_ignore).is_err());
}

#[test]
fn related_report_matches_route_and_selector_edges() {
    let root = Path::new("/repo");
    let edges = vec![
        Edge::Route {
            test_file: "tests/e2e/route.spec.ts".to_string(),
            test_name: None,
            describe_path: vec![],
            route_file: "web/app/page.tsx".to_string(),
            route: "/".to_string(),
            url: "/".to_string(),
        },
        Edge::Selector {
            test_file: "tests/e2e/selector.spec.ts".to_string(),
            test_name: None,
            describe_path: vec![],
            app_file: "web/app/components/save.tsx".to_string(),
            attribute: "data-testid".to_string(),
            value: "save".to_string(),
            selector: "getByTestId(save)".to_string(),
        },
    ];
    let report = build_related_report(
        root,
        &edges,
        &[
            PathBuf::from("/repo/web/app/page.tsx"),
            PathBuf::from("./web/app/components/save.tsx"),
        ],
    );

    assert_eq!(
        report.tests,
        vec!["tests/e2e/route.spec.ts", "tests/e2e/selector.spec.ts"]
    );
    print_related_text(&report);
}
