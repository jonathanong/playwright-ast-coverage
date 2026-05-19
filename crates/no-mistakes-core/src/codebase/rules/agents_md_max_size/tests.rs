use super::*;
use crate::config::v2::NoMistakesConfig;

fn config_with_rule(yaml: &str) -> NoMistakesConfig {
    let mut config = NoMistakesConfig::default();
    config
        .rules
        .insert(RULE_ID.to_string(), serde_yaml::from_str(yaml).unwrap());
    config
}

#[test]
fn count_lines_empty() {
    assert_eq!(count_lines(""), 0);
}

#[test]
fn count_lines_single_no_newline() {
    assert_eq!(count_lines("hello"), 1);
}

#[test]
fn count_lines_with_trailing_newline() {
    assert_eq!(count_lines("a\nb\n"), 2);
}

#[test]
fn count_lines_no_trailing_newline() {
    assert_eq!(count_lines("a\nb"), 2);
}

#[test]
fn count_lines_multibyte() {
    assert_eq!(count_lines("héllo\nwörld\n"), 2);
}

#[test]
fn check_file_passes_within_limits() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("AGENTS.md");
    std::fs::write(&path, "line1\nline2\n").unwrap();
    let findings = check_file(&path, tmp.path(), 10, 1000);
    assert!(findings.is_empty());
}

#[test]
fn check_file_fails_line_count() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("AGENTS.md");
    std::fs::write(&path, "a\nb\nc\n").unwrap();
    let findings = check_file(&path, tmp.path(), 2, 10000);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("3 lines"));
}

#[test]
fn check_file_fails_char_count() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("CLAUDE.md");
    std::fs::write(&path, "hello world").unwrap();
    let findings = check_file(&path, tmp.path(), 100, 5);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("characters"));
}

#[test]
fn check_file_fails_both() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("AGENTS.md");
    std::fs::write(&path, "abc\ndef\nghi\n").unwrap();
    let findings = check_file(&path, tmp.path(), 1, 1);
    assert_eq!(findings.len(), 2);
}

#[test]
fn check_file_respects_disable_file_comment() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("AGENTS.md");
    let content = format!("// guardrails-disable-file {RULE_ID}\na\nb\nc\n");
    std::fs::write(&path, content).unwrap();
    let findings = check_file(&path, tmp.path(), 2, 10000);
    assert!(findings.is_empty());
}

#[test]
fn check_file_multibyte_chars() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("AGENTS.md");
    std::fs::write(&path, "héllo").unwrap();
    let findings = check_file(&path, tmp.path(), 100, 3);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("5 characters"));
}

#[test]
fn check_uses_custom_options() {
    let tmp = tempfile::tempdir().unwrap();
    let agents_path = tmp.path().join("AGENTS.md");
    std::fs::write(&agents_path, "a\nb\nc\n").unwrap();
    let config = config_with_rule("{maxLines: 5, maxChars: 1000}");
    let findings = check(tmp.path(), &config).unwrap();
    assert!(findings.is_empty());
}

#[test]
fn check_uses_custom_filenames() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("GEMINI.md");
    std::fs::write(&path, "a\nb\nc\n").unwrap();
    // Without custom filename, GEMINI.md is not checked
    let config = config_with_rule("{maxLines: 2}");
    let findings = check(tmp.path(), &config).unwrap();
    assert!(findings.is_empty(), "GEMINI.md not in default set");
    // With custom filename
    let config2 = config_with_rule("{maxLines: 2, filenames: [\"GEMINI.md\"]}");
    let findings2 = check(tmp.path(), &config2).unwrap();
    assert_eq!(findings2.len(), 1);
}

#[test]
fn check_returns_empty_when_no_files() {
    let tmp = tempfile::tempdir().unwrap();
    let config = config_with_rule("{}");
    let findings = check(tmp.path(), &config).unwrap();
    assert!(findings.is_empty());
}

#[test]
fn check_sorts_findings_deterministically() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("AGENTS.md"), "a\nb\nc\n").unwrap();
    std::fs::write(tmp.path().join("CLAUDE.md"), "x\ny\nz\n").unwrap();
    let config = config_with_rule("{maxLines: 1, maxChars: 1}");
    let findings = check(tmp.path(), &config).unwrap();
    // findings should be sorted by file then message
    for i in 1..findings.len() {
        let a = (&findings[i - 1].file, &findings[i - 1].message);
        let b = (&findings[i].file, &findings[i].message);
        assert!(a <= b, "findings not sorted: {:?} > {:?}", a, b);
    }
}
