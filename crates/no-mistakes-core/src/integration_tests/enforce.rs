use super::types::{EffectiveIntegrationPolicy, IntegrationFinding, Suite, TestCase};
use crate::codebase::ts_source::relative_slash_path;
use std::path::Path;

pub(super) fn enforce_policy(
    root: &Path,
    suite: &Suite,
    test: &TestCase,
    integrations: &[String],
) -> Vec<IntegrationFinding> {
    match &suite.policy {
        EffectiveIntegrationPolicy::Disabled => integrations
            .iter()
            .map(|name| {
                finding(
                    root,
                    suite,
                    test,
                    Some(name.clone()),
                    disabled_message(suite, name),
                )
            })
            .collect(),
        EffectiveIntegrationPolicy::Suites { suites, strict } => {
            enforce_suite_policy(root, suite, test, integrations, suites, *strict)
        }
    }
}

fn enforce_suite_policy(
    root: &Path,
    suite: &Suite,
    test: &TestCase,
    integrations: &[String],
    suites: &[String],
    strict: bool,
) -> Vec<IntegrationFinding> {
    let mut findings = Vec::new();
    for name in integrations {
        if suites.contains(name) {
            continue;
        }
        findings.push(finding(
            root,
            suite,
            test,
            Some(name.clone()),
            format!(
                "{} suite {} allows only integration={}; found integration={name}",
                suite.framework.as_str(),
                suite.name,
                suites.join(",")
            ),
        ));
    }
    if findings.is_empty() && integrations.is_empty() && strict {
        findings.push(finding(
            root,
            suite,
            test,
            None,
            format!(
                "{} suite {} requires integration={}",
                suite.framework.as_str(),
                suite.name,
                suites.join(",")
            ),
        ));
    }
    findings
}

fn disabled_message(suite: &Suite, name: &str) -> String {
    format!(
        "{} suite {} does not allow integration tests; found integration={name}",
        suite.framework.as_str(),
        suite.name
    )
}

fn finding(
    root: &Path,
    suite: &Suite,
    test: &TestCase,
    integration: Option<String>,
    message: String,
) -> IntegrationFinding {
    IntegrationFinding {
        framework: suite.framework.as_str().to_string(),
        suite: suite.name.clone(),
        file: relative_slash_path(root, &test.function_key.file),
        line: test.line,
        test_name: test.name.clone(),
        describe_path: test.describe_path.clone(),
        integration,
        message,
    }
}
