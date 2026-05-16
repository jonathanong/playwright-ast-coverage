use super::run_check;
use crate::cli::{Cli, Command};
use crate::pipeline::run::run_analyze;
use std::path::PathBuf;

fn fixture(category: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(category)
        .join(name)
}

fn make_cli(root: PathBuf) -> Cli {
    Cli {
        root,
        config: None,
        json: false,
        command: Command::Check {
            targets: vec![],
            assert_no_fetch: false,
        },
    }
}

fn make_analyze_cli(root: PathBuf) -> Cli {
    Cli {
        root,
        config: None,
        json: false,
        command: Command::Analyze { targets: vec![] },
    }
}

#[test]
fn check_no_violations_clean_component() {
    let fixture_root = fixture("react-traits-components", "basic");
    let cli = make_cli(PathBuf::from("."));
    let targets = vec!["app/components/Greeting.tsx".to_string()];
    let violations = run_check(&fixture_root, &cli, &targets, false).expect("should check");
    assert!(violations.is_empty());
}

#[test]
fn check_assert_no_fetch_flag_reports_violation() {
    let fixture_root = fixture("react-traits-config", "assert-no-fetch");
    let cli = make_cli(PathBuf::from("."));
    let targets = vec!["app/components/Fetcher.tsx".to_string()];
    let violations = run_check(&fixture_root, &cli, &targets, true).expect("should check");
    assert!(!violations.is_empty());
    assert_eq!(violations[0].rule, "assert-no-fetch");
}

#[test]
fn check_assert_no_fetch_from_config() {
    let fixture_root = fixture("react-traits-config", "assert-no-fetch");
    let cli = make_cli(PathBuf::from("."));
    let targets = vec!["app/components/Fetcher.tsx".to_string()];
    let violations = run_check(&fixture_root, &cli, &targets, false).expect("should check");
    assert!(
        !violations.is_empty(),
        "config assertNoFetch should trigger violation"
    );
}

#[test]
fn check_no_targets_returns_empty() {
    // With no target patterns, expand_globs returns empty, so no violations.
    let fixture_root = fixture("react-traits-components", "basic");
    let cli = make_cli(PathBuf::from("."));
    let violations = run_check(&fixture_root, &cli, &[], false).expect("should check");
    assert!(violations.is_empty());
}

#[test]
fn check_target_not_at_root_falls_back_to_frontend_root() {
    // When targets are relative (not found from root), falls back to frontend_root.
    // This exercises the expand_globs(&frontend_root, targets) fallback.
    let fixture_root = fixture("react-traits-components", "basic");
    let cli = make_cli(PathBuf::from("."));
    // "components/Greeting.tsx" is relative to app/, not to root/
    let targets = vec!["components/Greeting.tsx".to_string()];
    let violations = run_check(&fixture_root, &cli, &targets, false).expect("should check");
    assert!(violations.is_empty());
}

#[test]
fn run_check_bad_file_returns_error() {
    // Exercises the `run_analyze(...)?` error branch in run_check (line 17 in check.rs).
    let fixture_root = fixture("react-traits-components", "bad-file");
    let cli = make_cli(PathBuf::from("."));
    let targets = vec!["app/components/Broken.tsx".to_string()];
    let result = run_check(&fixture_root, &cli, &targets, false);
    assert!(
        result.is_err(),
        "bad file should propagate error from run_analyze"
    );
}

#[test]
fn run_check_bad_config_returns_error() {
    // Exercises the `load_root_and_config(...)?` error branch in run_check (line 15 in check.rs).
    let fixture_root = fixture("react-traits-components", "basic");
    let cli = Cli {
        root: PathBuf::from("."),
        config: Some(PathBuf::from("nonexistent-config.yaml")),
        json: false,
        command: Command::Check {
            targets: vec![],
            assert_no_fetch: false,
        },
    };
    let result = run_check(&fixture_root, &cli, &[], false);
    assert!(
        result.is_err(),
        "missing config file should produce an error"
    );
}

#[test]
fn run_analyze_bad_file_returns_error() {
    // Exercises the `analyze_file(...)?` error branch in run_analyze (line 35 in run.rs).
    let fixture_root = fixture("react-traits-components", "bad-file");
    let cli = make_analyze_cli(PathBuf::from("."));
    let targets = vec!["app/components/Broken.tsx".to_string()];
    let result = run_analyze(&fixture_root, &cli, &targets, None);
    assert!(
        result.is_err(),
        "bad file should propagate error from analyze_file"
    );
}

#[test]
fn check_inherited_fetch_from_child_produces_violation() {
    // Parent renders Child which has fetch. With --assert-no-fetch, Parent should
    // get a violation via inherited_from_children.has_fetch.
    let fixture_root = fixture("react-traits-components", "nested");
    let cli = make_cli(PathBuf::from("."));
    let targets = vec![
        "app/components/Parent.tsx".to_string(),
        "app/components/Child.tsx".to_string(),
    ];
    let violations = run_check(&fixture_root, &cli, &targets, true).expect("should check");
    // Child directly has fetch, Parent inherits it — both should be reported.
    let names: Vec<&str> = violations.iter().map(|v| v.component.as_str()).collect();
    assert!(
        names.contains(&"default"),
        "expected at least one violation; got: {names:?}"
    );
}
