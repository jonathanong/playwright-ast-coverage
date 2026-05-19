use super::*;
use crate::config::v2::NoMistakesConfig;
use std::path::Path;

fn config_with_rule(yaml: &str) -> NoMistakesConfig {
    let mut config = NoMistakesConfig::default();
    config
        .rules
        .insert(RULE_ID.to_string(), serde_yaml::from_str(yaml).unwrap());
    config
}

#[test]
fn count_code_lines_empty() {
    assert_eq!(count_code_lines(""), 0);
}

#[test]
fn count_code_lines_blank_and_comment_only() {
    let src = "\n  // comment\n   \n";
    assert_eq!(count_code_lines(src), 0);
}

#[test]
fn count_code_lines_simple() {
    let src = "fn foo() {\n    let x = 1;\n}\n";
    assert_eq!(count_code_lines(src), 3);
}

#[test]
fn count_code_lines_block_comment_multiline() {
    let src = "/* start\n middle\n end */\nlet x = 1;\n";
    assert_eq!(count_code_lines(src), 1);
}

#[test]
fn count_code_lines_nested_block_comment() {
    // Rust supports nested block comments; the entire first line is a comment.
    let src = "/* outer /* inner */ still outer */\ncode\n";
    assert_eq!(count_code_lines(src), 1);
}

#[test]
fn count_code_lines_inline_block_comment() {
    let src = "let x = /* comment */ 1;\n";
    assert_eq!(count_code_lines(src), 1);
}

#[test]
fn count_code_lines_doc_comment() {
    let src = "/// doc comment\nfn foo() {}\n";
    assert_eq!(count_code_lines(src), 1);
}

#[test]
fn is_test_file_tests_dir() {
    let root = Path::new("/project");
    let path = Path::new("/project/crates/foo/tests/integration.rs");
    assert!(is_test_file(root, path));
}

#[test]
fn is_test_file_tests_rs_basename() {
    let root = Path::new("/project");
    let path = Path::new("/project/crates/foo/src/module/tests.rs");
    assert!(is_test_file(root, path));
}

#[test]
fn is_test_file_src_file() {
    let root = Path::new("/project");
    let path = Path::new("/project/crates/foo/src/lib.rs");
    assert!(!is_test_file(root, path));
}

#[test]
fn check_passes_within_src_limit() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("lib.rs");
    let content = "fn a() {}\nfn b() {}\n";
    std::fs::write(&path, content).unwrap();
    let config = config_with_rule("{srcMax: 5}");
    let findings = check(tmp.path(), &config).unwrap();
    assert!(findings.is_empty());
}

#[test]
fn check_fails_over_src_limit() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("big.rs");
    let content = (0..10)
        .map(|i| format!("fn f{i}() {{}}\n"))
        .collect::<String>();
    std::fs::write(&path, content).unwrap();
    let config = config_with_rule("{srcMax: 3}");
    let findings = check(tmp.path(), &config).unwrap();
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("code lines"));
}

#[test]
fn check_uses_test_limit_for_tests_rs() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("tests.rs");
    let content = (0..10)
        .map(|i| format!("fn f{i}() {{}}\n"))
        .collect::<String>();
    std::fs::write(&path, content).unwrap();
    let config = config_with_rule("{srcMax: 3, testMax: 20}");
    let findings = check(tmp.path(), &config).unwrap();
    assert!(findings.is_empty(), "tests.rs should use testMax");
}

#[test]
fn check_respects_excludes() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("generated.rs");
    let content = (0..10)
        .map(|i| format!("fn f{i}() {{}}\n"))
        .collect::<String>();
    std::fs::write(&path, content).unwrap();
    let config = config_with_rule("{srcMax: 3, excludes: [\"generated\"]}");
    let findings = check(tmp.path(), &config).unwrap();
    assert!(findings.is_empty());
}

#[test]
fn check_file_skips_unreadable_file() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("missing.rs");
    // Path does not exist → read_to_string fails → returns None
    let finding = check_file(&path, tmp.path(), 5);
    assert!(finding.is_none());
}

#[test]
fn check_respects_disable_file_comment() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("big.rs");
    let content = format!(
        "// guardrails-disable-file {RULE_ID}\n{}",
        (0..10)
            .map(|i| format!("fn f{i}() {{}}\n"))
            .collect::<String>()
    );
    std::fs::write(&path, content).unwrap();
    let config = config_with_rule("{srcMax: 3}");
    let findings = check(tmp.path(), &config).unwrap();
    assert!(findings.is_empty());
}
