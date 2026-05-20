use super::*;
use crate::check_parallel::DomainResults;
use crate::check_tasks::CheckTask;
use anyhow::anyhow;
use no_mistakes_core::codebase::rules::{
    RuleFinding, RUST_MAX_LINES_PER_FILE, RUST_NO_INLINE_TESTS,
};
use no_mistakes_core::codebase::unique_exports::UniqueExportFinding;
use no_mistakes_core::integration_tests::IntegrationFinding;
use no_mistakes_core::queue::CheckFinding;
use no_mistakes_core::react_traits;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn run_all_keeps_filesystem_files_when_fact_collection_is_needed() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/check-runner/facts-and-filesystem");
    let config = root.join(".no-mistakes.yml");

    let results = run_all(root, Some(config), None).unwrap();

    assert!(results.has_findings());
    assert!(results
        .rules
        .iter()
        .any(|finding| finding.rule == RUST_MAX_LINES_PER_FILE));
    assert_eq!(results.rules.len(), 2);
    let mut rule_ids = vec![
        results.rules[0].rule.as_str(),
        results.rules[1].rule.as_str(),
    ];
    rule_ids.sort();
    assert_eq!(
        rule_ids,
        vec![RUST_MAX_LINES_PER_FILE, RUST_NO_INLINE_TESTS]
    );
}

#[test]
fn run_codebase_check_uses_explicit_tsconfig_with_shared_facts() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis/unique-exports-basic");
    let config = root.join(".no-mistakes.yml");
    let files = no_mistakes_core::codebase::ts_source::discover_files(&root, &[]);
    let facts = no_mistakes_core::codebase::check_facts::collect_check_facts(
        &root,
        files,
        no_mistakes_core::codebase::check_facts::CheckFactPlan {
            source: true,
            symbols: true,
            ..Default::default()
        },
    );

    let results = crate::check_tasks::run_codebase_check(
        root.clone(),
        Some(config),
        Some(root.join("tsconfig.json")),
        true,
        &facts,
    )
    .unwrap();

    assert!(!results.findings.is_empty());
}

#[test]
fn run_codebase_check_propagates_unique_exports_errors() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis/unique-exports-basic");
    let facts = no_mistakes_core::codebase::check_facts::CheckFactMap::default();

    let error = crate::check_tasks::run_codebase_check(
        root.clone(),
        Some(root.join("missing.no-mistakes.yml")),
        None,
        true,
        &facts,
    )
    .err()
    .expect("expected missing config error");

    assert!(error.to_string().contains("missing.no-mistakes.yml"));
}

#[test]
fn run_all_surfaces_react_enabled_config_errors() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/check-runner/react-config-error");
    let config = root.join(".no-mistakes.yml");

    let err = run_all(root, Some(config), None)
        .err()
        .expect("expected react config error");

    assert!(err.to_string().contains("failed to parse"));
}

#[test]
fn integration_configured_covers_vitest_and_playwright_suites() {
    let empty = no_mistakes_core::config::v2::NoMistakesConfig::default();
    assert!(!integration_configured(&empty));

    let mut vitest = no_mistakes_core::config::v2::NoMistakesConfig::default();
    vitest.tests.vitest.projects.insert(
        "web".to_string(),
        no_mistakes_core::config::v2::schema::TestProjectPolicy {
            integration_suites: BTreeMap::from([(
                "openai".to_string(),
                vec!["openai".to_string()],
            )]),
        },
    );
    assert!(integration_configured(&vitest));

    let mut playwright = no_mistakes_core::config::v2::NoMistakesConfig::default();
    playwright.tests.playwright.projects.insert(
        "e2e".to_string(),
        no_mistakes_core::config::v2::schema::TestProjectPolicy {
            integration_suites: BTreeMap::from([("aws".to_string(), vec!["aws".to_string()])]),
        },
    );
    assert!(integration_configured(&playwright));
}

#[test]
fn complete_domain_checks_surfaces_each_domain_error() {
    assert_domain_error(err_react(), "react");
    assert_domain_error(err_queues(), "queues");
    assert_domain_error(err_rules(), "rules");
    assert_domain_error(err_integration(), "integration");
    assert_domain_error(err_codebase(), "codebase");
    assert_domain_error(err_filesystem_rules(), "filesystem_rules");
}

fn assert_domain_error(results: DomainResults, expected: &str) {
    let err = complete_domain_checks(results)
        .err()
        .expect("expected domain check error");
    assert_eq!(err.to_string(), expected);
}

fn empty_task<T>(findings: T) -> CheckTask<T> {
    CheckTask {
        findings,
        warning: None,
        duration: Duration::ZERO,
    }
}

fn ok_react() -> anyhow::Result<CheckTask<Vec<react_traits::Violation>>> {
    Ok(empty_task(Vec::new()))
}

fn ok_queues() -> anyhow::Result<CheckTask<Vec<CheckFinding>>> {
    Ok(empty_task(Vec::new()))
}

fn ok_rules() -> anyhow::Result<CheckTask<Vec<RuleFinding>>> {
    Ok(empty_task(Vec::new()))
}

fn ok_integration() -> anyhow::Result<CheckTask<Vec<IntegrationFinding>>> {
    Ok(empty_task(Vec::new()))
}

fn ok_codebase() -> anyhow::Result<CheckTask<Vec<UniqueExportFinding>>> {
    Ok(empty_task(Vec::new()))
}

fn err_react() -> DomainResults {
    (
        Err(anyhow!("react")),
        ok_queues(),
        ok_rules(),
        ok_integration(),
        ok_codebase(),
        ok_rules(),
    )
}

fn err_queues() -> DomainResults {
    (
        ok_react(),
        Err(anyhow!("queues")),
        ok_rules(),
        ok_integration(),
        ok_codebase(),
        ok_rules(),
    )
}

fn err_rules() -> DomainResults {
    (
        ok_react(),
        ok_queues(),
        Err(anyhow!("rules")),
        ok_integration(),
        ok_codebase(),
        ok_rules(),
    )
}

fn err_integration() -> DomainResults {
    (
        ok_react(),
        ok_queues(),
        ok_rules(),
        Err(anyhow!("integration")),
        ok_codebase(),
        ok_rules(),
    )
}

fn err_codebase() -> DomainResults {
    (
        ok_react(),
        ok_queues(),
        ok_rules(),
        ok_integration(),
        Err(anyhow!("codebase")),
        ok_rules(),
    )
}

fn err_filesystem_rules() -> DomainResults {
    (
        ok_react(),
        ok_queues(),
        ok_rules(),
        ok_integration(),
        ok_codebase(),
        Err(anyhow!("filesystem_rules")),
    )
}
