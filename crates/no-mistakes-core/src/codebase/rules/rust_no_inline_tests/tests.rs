use super::*;
use crate::config::v2::NoMistakesConfig;

fn config_with_rule(yaml: &str) -> NoMistakesConfig {
    let mut config = NoMistakesConfig::default();
    config.rules.insert(
        RULE_ID.to_string(),
        serde_yaml::from_str(yaml).unwrap(),
    );
    config
}

fn check_source(source: &str) -> Vec<RuleFinding> {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("lib.rs");
    std::fs::write(&path, source).unwrap();
    let root = tmp.path();
    check_file(&path, root)
}

#[test]
fn no_match_on_clean_source() {
    let src = "fn foo() {}\n";
    assert!(check_source(src).is_empty());
}

#[test]
fn no_match_on_out_of_line_tests() {
    let src = "#[cfg(test)]\nmod tests;\n";
    assert!(
        check_source(src).is_empty(),
        "out-of-line mod tests; must not be flagged"
    );
}

#[test]
fn matches_simple_inline() {
    let src = "#[cfg(test)]\nmod tests {\n    fn it_works() {}\n}\n";
    let findings = check_source(src);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].line, 1);
}

#[test]
fn matches_inline_on_one_line() {
    let src = "#[cfg(test)] mod tests { fn it_works() {} }\n";
    let findings = check_source(src);
    assert_eq!(findings.len(), 1);
}

#[test]
fn matches_pub_mod() {
    let src = "#[cfg(test)]\npub mod tests {\n    fn it_works() {}\n}\n";
    let findings = check_source(src);
    assert_eq!(findings.len(), 1);
}

#[test]
fn matches_with_whitespace_in_cfg() {
    let src = "# [ cfg ( test ) ]\nmod tests {}\n";
    let findings = check_source(src);
    assert_eq!(findings.len(), 1);
}

#[test]
fn matches_with_extra_attributes() {
    let src = "#[cfg(test)]\n#[allow(clippy::all)]\nmod tests {\n}\n";
    let findings = check_source(src);
    assert_eq!(findings.len(), 1);
}

#[test]
fn respects_disable_file_comment() {
    let src = format!("// guardrails-disable-file {RULE_ID}\n#[cfg(test)]\nmod tests {{\n}}\n");
    let findings = check_source(&src);
    assert!(findings.is_empty());
}

#[test]
fn check_returns_empty_for_no_rs_files() {
    let tmp = tempfile::tempdir().unwrap();
    let config = config_with_rule("{}");
    let findings = check(tmp.path(), &config).unwrap();
    assert!(findings.is_empty());
}

#[test]
fn check_reports_correct_file_path() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("mymod.rs");
    std::fs::write(&path, "#[cfg(test)]\nmod tests {\n}\n").unwrap();
    let config = config_with_rule("{}");
    let findings = check(tmp.path(), &config).unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].file, "mymod.rs");
}

#[test]
fn check_respects_excludes() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("generated.rs");
    std::fs::write(&path, "#[cfg(test)]\nmod tests {\n}\n").unwrap();
    let config = config_with_rule("{excludes: [\"generated\"]}");
    let findings = check(tmp.path(), &config).unwrap();
    assert!(findings.is_empty());
}

#[test]
fn check_sorts_by_file_then_line() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("a.rs"),
        "#[cfg(test)]\nmod a_tests {}\n#[cfg(test)]\nmod b_tests {}\n",
    )
    .unwrap();
    std::fs::write(
        tmp.path().join("b.rs"),
        "#[cfg(test)]\nmod tests {}\n",
    )
    .unwrap();
    let config = config_with_rule("{}");
    let findings = check(tmp.path(), &config).unwrap();
    assert_eq!(findings.len(), 3);
    for i in 1..findings.len() {
        let a = (&findings[i - 1].file, findings[i - 1].line);
        let b = (&findings[i].file, findings[i].line);
        assert!(a <= b);
    }
}
