use super::*;

fn nonexistent_config() -> std::path::PathBuf {
    std::path::PathBuf::from("/nonexistent/config.yaml")
}

fn assert_no_fetch_root() -> std::path::PathBuf {
    crate::codebase::ts_resolver::normalize_path(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/react-traits-config/assert-no-fetch"),
    )
}

#[test]
fn run_check_with_facts_skips_when_assert_no_fetch_is_disabled() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/react-traits-components/nested");
    let facts = crate::codebase::check_facts::CheckFactMap::default();

    let findings = run_check_with_facts(
        &root,
        None,
        &["app/components/Child.tsx".to_string()],
        false,
        &facts,
    )
    .unwrap();

    assert!(findings.is_empty());
}

#[test]
fn run_check_reports_violations_when_assert_no_fetch_is_enabled() {
    let root = assert_no_fetch_root();
    let violations = run_check(&root, None, &[], true).unwrap();
    assert!(!violations.is_empty(), "expected fetch violations");
}

#[test]
fn check_enabled_returns_true_when_assert_no_fetch_is_enabled() {
    let root = assert_no_fetch_root();
    assert!(check_enabled(&root, None, true).unwrap());
}

#[test]
fn check_enabled_returns_false_when_assert_no_fetch_is_disabled() {
    // Fixture with no assert_no_fetch config
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/react-traits-components/nested");
    assert!(!check_enabled(&root, None, false).unwrap());
}

#[test]
fn run_check_returns_error_for_nonexistent_config_path() {
    let root = assert_no_fetch_root();
    let config = nonexistent_config();
    let err = run_check(&root, Some(&config), &[], false);
    assert!(err.is_err(), "expected error for nonexistent config");
}

#[test]
fn run_check_with_facts_returns_error_for_nonexistent_config_path() {
    let root = assert_no_fetch_root();
    let config = nonexistent_config();
    let facts = crate::codebase::check_facts::CheckFactMap::default();
    let err = run_check_with_facts(&root, Some(&config), &[], false, &facts);
    assert!(err.is_err(), "expected error for nonexistent config");
}

#[test]
fn check_enabled_returns_error_for_nonexistent_config_path() {
    let root = assert_no_fetch_root();
    let config = nonexistent_config();
    let err = check_enabled(&root, Some(&config), false);
    assert!(err.is_err(), "expected error for nonexistent config");
}

#[test]
fn run_check_with_facts_reports_violations_when_assert_no_fetch_is_enabled() {
    use crate::codebase::check_facts::{collect_check_facts, CheckFactPlan};

    let root = assert_no_fetch_root();
    let fetcher = root.join("app/components/Fetcher.tsx");
    let plan = CheckFactPlan {
        react: true,
        ..CheckFactPlan::default()
    };
    let facts = collect_check_facts(&root, vec![fetcher], plan);
    let violations = run_check_with_facts(&root, None, &[], true, &facts).unwrap();
    assert!(!violations.is_empty(), "expected fetch violations");
}
