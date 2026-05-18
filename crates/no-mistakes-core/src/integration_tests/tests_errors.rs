use super::*;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/integration-tests")
            .join(name),
    )
}

fn fixture_file(file: &str) -> PathBuf {
    fixture("parse-errors").join(file)
}

#[test]
fn analyzer_reports_parse_context() {
    let file = fixture_file("src/syntax-error.ts");
    let err = analysis::analyze_files(&[file])
        .err()
        .expect("expected syntax error");
    assert!(err
        .to_string()
        .contains("analyzing integration annotations"));
}

#[test]
fn config_parsers_report_syntax_errors() {
    let root = fixture("parse-errors");

    let pw_path = root.join("playwright.syntax-error.ts");
    let pw_source = std::fs::read_to_string(&pw_path).unwrap();
    assert!(test_config::playwright::parse_from_path(&pw_source, &pw_path, &root).is_err());

    let vitest_path = root.join("vitest.syntax-error.mts");
    let vitest_source = std::fs::read_to_string(&vitest_path).unwrap();
    assert!(
        test_config::vitest::parse_from_path(&vitest_source, &vitest_path, &root, &root).is_err()
    );

    let root = fixture("coverage");
    let empty_path = root.join("vitest.empty-array-invalid.mts");
    let empty_source = std::fs::read_to_string(&empty_path).unwrap();
    assert!(
        test_config::vitest::parse_from_path(&empty_source, &empty_path, &root, &root).is_err()
    );
}

#[test]
fn check_with_facts_reports_dropped_helper_parse_errors() {
    let root = fixture("basic");
    let file = root.join("helpers/openai.mts");
    let mut shared = crate::codebase::check_facts::CheckFactMap::default();
    shared.ts.insert(
        file,
        crate::codebase::check_facts::CheckFileFacts {
            parse_error: Some("synthetic helper parse error".to_string()),
            ..Default::default()
        },
    );

    let error = check_with_facts(&root, None, &shared).unwrap_err();

    assert!(error.to_string().contains("synthetic helper parse error"));
}
