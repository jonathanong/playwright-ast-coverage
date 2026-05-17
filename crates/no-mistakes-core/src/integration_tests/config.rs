use super::project_config;
use super::types::{ConfigProject, EffectiveIntegrationPolicy, Framework, Suite};
use crate::config::v2::schema::{
    IntegrationPolicy, IntegrationSuitesPolicy, NoMistakesConfig, StringOrList, TestSuitePolicy,
};
use anyhow::Result;
use std::path::Path;

const DEFAULT_TEST_GLOBS: &[&str] = &[
    "**/*.test.ts",
    "**/*.test.tsx",
    "**/*.test.js",
    "**/*.test.jsx",
    "**/*.test.mts",
    "**/*.test.cts",
    "**/*.test.mjs",
    "**/*.test.cjs",
    "**/*.spec.ts",
    "**/*.spec.tsx",
    "**/*.spec.js",
    "**/*.spec.jsx",
    "**/*.spec.mts",
    "**/*.spec.cts",
    "**/*.spec.mjs",
    "**/*.spec.cjs",
];

pub(super) fn validate_config(config: &NoMistakesConfig) -> Result<()> {
    for (framework, suites) in [
        ("playwright", config.tests.playwright.suites.as_slice()),
        ("vitest", config.tests.vitest.suites.as_slice()),
    ] {
        for suite in suites {
            match &suite.integration {
                IntegrationPolicy::Disabled(false) => {}
                IntegrationPolicy::Disabled(true) => anyhow::bail!(
                    "tests.{framework}.suites integration: true is not supported; use integration.suites"
                ),
                IntegrationPolicy::Suites(policy) if policy.suites.is_empty() => anyhow::bail!(
                    "tests.{framework}.suites integration.suites must contain at least one name"
                ),
                IntegrationPolicy::Suites(_) => {}
            }
        }
    }
    Ok(())
}

pub(super) fn configured_suites(root: &Path, config: &NoMistakesConfig) -> Result<Vec<Suite>> {
    let mut suites = Vec::new();
    suites.extend(suites_for_framework(
        root,
        Framework::Playwright,
        config.tests.playwright.configs.as_ref(),
        &config.tests.playwright.suites,
    )?);
    suites.extend(suites_for_framework(
        root,
        Framework::Vitest,
        config.tests.vitest.configs.as_ref(),
        &config.tests.vitest.suites,
    )?);
    Ok(suites)
}

fn suites_for_framework(
    root: &Path,
    framework: Framework,
    configs: Option<&StringOrList>,
    policies: &[TestSuitePolicy],
) -> Result<Vec<Suite>> {
    let projects = project_config::load_projects(root, framework, configs)?;
    let mut suites = Vec::new();
    for (index, policy) in policies.iter().enumerate() {
        let name = policy
            .name
            .clone()
            .or_else(|| policy.project.clone())
            .unwrap_or_else(|| format!("suite-{}", index + 1));
        let (include, exclude) = suite_globs(framework, &name, policy, &projects)?;
        suites.push(Suite {
            framework,
            name,
            include,
            exclude,
            policy: effective_policy(&policy.integration),
        });
    }
    Ok(suites)
}

fn suite_globs(
    framework: Framework,
    name: &str,
    policy: &TestSuitePolicy,
    projects: &[ConfigProject],
) -> Result<(Vec<String>, Vec<String>)> {
    let mut include = policy.include.clone();
    let mut exclude = policy.exclude.clone();
    if policy.project.is_some() || policy.config.is_some() {
        let matched = matched_projects(policy, projects);
        if matched.is_empty() {
            anyhow::bail!(
                "{} suite {} references unknown {}",
                framework.as_str(),
                name,
                policy_target(policy)
            );
        }
        if include.is_empty() {
            include.extend(matched.iter().flat_map(|project| project.include.clone()));
        }
        exclude.extend(matched.iter().flat_map(|project| project.exclude.clone()));
    }
    if include.is_empty() {
        include = DEFAULT_TEST_GLOBS
            .iter()
            .map(|glob| glob.to_string())
            .collect();
    }
    Ok((include, exclude))
}

fn matched_projects<'a>(
    policy: &TestSuitePolicy,
    projects: &'a [ConfigProject],
) -> Vec<&'a ConfigProject> {
    projects
        .iter()
        .filter(|project| {
            policy
                .project
                .as_ref()
                .is_none_or(|name| project.name.as_deref() == Some(name.as_str()))
                && policy
                    .config
                    .as_ref()
                    .is_none_or(|config| project.config.as_deref() == Some(config.as_str()))
        })
        .collect()
}

pub(in crate::integration_tests) fn policy_target(policy: &TestSuitePolicy) -> String {
    match (&policy.config, &policy.project) {
        (Some(config), Some(project)) => format!("config {config} project {project}"),
        (Some(config), None) => format!("config {config}"),
        (None, Some(project)) => format!("project {project}"),
        (None, None) => "suite".to_string(),
    }
}

fn effective_policy(policy: &IntegrationPolicy) -> EffectiveIntegrationPolicy {
    match policy {
        IntegrationPolicy::Disabled(_) => EffectiveIntegrationPolicy::Disabled,
        IntegrationPolicy::Suites(IntegrationSuitesPolicy { suites, strict }) => {
            EffectiveIntegrationPolicy::Suites {
                suites: suites.clone(),
                strict: *strict,
            }
        }
    }
}
