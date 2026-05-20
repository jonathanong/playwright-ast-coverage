use crate::config::v2::NoMistakesConfig;
use anyhow::Result;
use std::path::Path;

pub(super) fn rule_test_project_globs(
    root: &Path,
    config: &NoMistakesConfig,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut includes = Vec::new();
    let mut excludes = Vec::new();
    let projects = crate::integration_tests::project_config::load_projects(
        root,
        crate::integration_tests::types::Framework::Vitest,
        config.tests.vitest.configs.as_ref(),
    )?;
    for rule in config.rule_applications(super::super::RULE_ID) {
        for project_name in &rule.tests.vitest {
            let Some(project) = projects
                .iter()
                .find(|project| project.name.as_deref() == Some(project_name.as_str()))
            else {
                anyhow::bail!("test-no-unmocked-dynamic-imports references unknown vitest project {project_name}");
            };
            includes.extend(project.include.clone());
            excludes.extend(project.exclude.clone());
        }
        for project_name in &rule.projects {
            append_project_includes(config, project_name, &mut includes);
        }
    }
    includes.sort();
    includes.dedup();
    excludes.sort();
    excludes.dedup();
    Ok((includes, excludes))
}

fn append_project_includes(
    config: &NoMistakesConfig,
    project_name: &str,
    includes: &mut Vec<String>,
) {
    let Some(project) = config.projects.get(project_name) else {
        return;
    };
    let root = project.root.as_deref().unwrap_or(".").trim_matches('/');
    let project_includes = if project.include.is_empty() {
        vec!["**".to_string()]
    } else {
        project.include.clone()
    };
    for include in project_includes {
        if root.is_empty() || root == "." {
            includes.push(include);
        } else {
            includes.push(format!(
                "{}/{}",
                root.trim_start_matches("./"),
                include.trim_start_matches("./")
            ));
        }
    }
}
