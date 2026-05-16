use super::run_check;
use crate::cli::{Cli, Command};
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
    // This exercises the expand_globs(&frontend_root, targets) fallback (line 30).
    let fixture_root = fixture("react-traits-components", "basic");
    let cli = make_cli(PathBuf::from("."));
    // "components/Greeting.tsx" is relative to app/, not to root/
    let targets = vec!["components/Greeting.tsx".to_string()];
    let violations = run_check(&fixture_root, &cli, &targets, false).expect("should check");
    assert!(violations.is_empty());
}
