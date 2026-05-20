use super::project_config;
use super::types::{ConfigProject, EffectiveIntegrationPolicy, Framework, Suite};
use crate::config::v2::schema::{NoMistakesConfig, StringOrList, TestProjectPolicy};
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn validate_config(config: &NoMistakesConfig) -> Result<()> {
    for (framework, projects) in [
        ("playwright", &config.tests.playwright.projects),
        ("vitest", &config.tests.vitest.projects),
    ] {
        for (project, policy) in projects {
            for (suite, integrations) in &policy.integration_suites {
                if integrations.is_empty() {
                    anyhow::bail!(
                        "tests.{framework}.projects.{project}.integration_suites.{suite} must contain at least one integration"
                    );
                }
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
        &config.tests.playwright.projects,
    )?);
    suites.extend(suites_for_framework(
        root,
        Framework::Vitest,
        config.tests.vitest.configs.as_ref(),
        &config.tests.vitest.projects,
    )?);
    Ok(suites)
}

fn suites_for_framework(
    root: &Path,
    framework: Framework,
    configs: Option<&StringOrList>,
    policies: &BTreeMap<String, TestProjectPolicy>,
) -> Result<Vec<Suite>> {
    if policies.is_empty() {
        return Ok(Vec::new());
    }

    let projects = project_config::load_projects(root, framework, configs)?;
    let mut suites = Vec::new();
    for (project_name, policy) in policies {
        let project = exact_project(framework, project_name, &projects)?;
        for (suite_name, integrations) in &policy.integration_suites {
            suites.push(Suite {
                framework,
                name: format!("{project_name}.{suite_name}"),
                include: project.include.clone(),
                exclude: project.exclude.clone(),
                policy: EffectiveIntegrationPolicy::Suites {
                    suites: integrations.clone(),
                },
            });
        }
    }
    Ok(suites)
}

fn exact_project<'a>(
    framework: Framework,
    project_name: &str,
    projects: &'a [ConfigProject],
) -> Result<&'a ConfigProject> {
    projects
        .iter()
        .find(|project| project.name.as_deref() == Some(project_name))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "{} integration policy references unknown project {}",
                framework.as_str(),
                project_name
            )
        })
}
