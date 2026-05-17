use super::*;

#[test]
fn rule_enabled_requires_configured_rule() {
    let mut config = crate::config::v2::NoMistakesConfig::default();
    assert!(!rule_enabled(&config, TEST_NO_UNMOCKED_DYNAMIC_IMPORTS));
    config.rules.insert(
        TEST_NO_UNMOCKED_DYNAMIC_IMPORTS.to_string(),
        serde_yaml::from_str("{}").unwrap(),
    );
    assert!(rule_enabled(&config, TEST_NO_UNMOCKED_DYNAMIC_IMPORTS));
}

#[test]
fn rule_enabled_accepts_project_rule_without_top_level_options() {
    let mut config = crate::config::v2::NoMistakesConfig::default();
    config.projects.insert(
        "tests".to_string(),
        crate::config::v2::schema::Project {
            rules: vec![TEST_NO_UNMOCKED_DYNAMIC_IMPORTS.to_string()],
            ..Default::default()
        },
    );
    assert!(rule_enabled(&config, TEST_NO_UNMOCKED_DYNAMIC_IMPORTS));
}

#[test]
fn run_check_returns_empty_when_rule_is_not_enabled() {
    let root = std::path::Path::new("/tmp/no-mistakes-empty-rules");
    let findings = run_check(root, None, None).unwrap();
    assert!(findings.is_empty());
}
