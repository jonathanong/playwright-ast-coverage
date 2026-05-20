use super::super::*;
use crate::config::v2::schema::{Project, RuleDef, RuleTestTargets, StringOrList};
use std::path::{Path, PathBuf};

fn root_fixture(path: &str) -> PathBuf {
    crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(path),
    )
}

#[test]
fn vitest_rule_target_uses_exact_project_globs() {
    let root = root_fixture("integration-tests/basic");
    let mut config = NoMistakesConfig::default();
    config.rules.push(RuleDef {
        rule: super::super::super::RULE_ID.to_string(),
        tests: RuleTestTargets {
            vitest: vec!["unit".to_string()],
            ..Default::default()
        },
        ..Default::default()
    });

    let filter = test_filter(&root, &config).unwrap();

    assert!(filter.is_match("backend/unit.test.mts"));
    assert!(!filter.is_match("integration/openai.test.mts"));
}

#[test]
fn vitest_rule_target_rejects_unknown_project() {
    let root = root_fixture("integration-tests/basic");
    let mut config = NoMistakesConfig::default();
    config.rules.push(RuleDef {
        rule: super::super::super::RULE_ID.to_string(),
        tests: RuleTestTargets {
            vitest: vec!["missing".to_string()],
            ..Default::default()
        },
        ..Default::default()
    });

    let error = match test_filter(&root, &config) {
        Ok(_) => panic!("expected unknown project error"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("unknown vitest project missing"));
}

#[test]
fn vitest_rule_target_reports_missing_explicit_config() {
    let root = root_fixture("integration-tests/basic");
    let mut config = NoMistakesConfig::default();
    config.tests.vitest.configs = Some(StringOrList::One("missing.vitest.config.mts".to_string()));
    config.rules.push(RuleDef {
        rule: super::super::super::RULE_ID.to_string(),
        tests: RuleTestTargets {
            vitest: vec!["unit".to_string()],
            ..Default::default()
        },
        ..Default::default()
    });

    let error = match test_filter(&root, &config) {
        Ok(_) => panic!("expected missing explicit config error"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("vitest config does not exist"));
}

#[test]
fn missing_project_rule_target_falls_back_to_config_globs() {
    let root = root_fixture("codebase-analysis/test-no-unmocked-dynamic-imports");
    let mut config = NoMistakesConfig::default();
    config.rules.push(RuleDef {
        rule: super::super::super::RULE_ID.to_string(),
        projects: vec!["missing".to_string()],
        ..Default::default()
    });

    let filter = test_filter(&root, &config).unwrap();

    assert!(filter.is_match("tests/good.test.mts"));
    assert!(!filter.is_match("src/unmocked-child.mts"));
}

#[test]
fn empty_project_rule_target_matches_everything_under_project_root() {
    let mut config = NoMistakesConfig::default();
    config
        .projects
        .insert("all".to_string(), Project::default());
    config.rules.push(RuleDef {
        rule: super::super::super::RULE_ID.to_string(),
        projects: vec!["all".to_string()],
        ..Default::default()
    });

    let filter = test_filter(Path::new("."), &config).unwrap();

    assert!(filter.is_match("src/not-a-test.ts"));
}
