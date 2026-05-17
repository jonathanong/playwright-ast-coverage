use super::test_config;
use super::types::{ConfigProject, Framework};
use crate::codebase::ts_resolver::{find_tsconfig, load_tsconfig, TsConfig};
use crate::codebase::ts_source::relative_slash_path;
use crate::config::v2::schema::StringOrList;
use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::Path;

pub(super) fn load_projects(
    root: &Path,
    framework: Framework,
    configs: Option<&StringOrList>,
) -> Result<Vec<ConfigProject>> {
    let Some(configs) = configs else {
        return Ok(Vec::new());
    };
    let mut projects = Vec::new();
    for raw in configs.values() {
        let path = root.join(&raw);
        if !path.exists() {
            anyhow::bail!(
                "{} config does not exist: {}",
                framework.as_str(),
                path.display()
            );
        }
        let source = std::fs::read_to_string(&path)?;
        let config_dir = path.parent().unwrap_or(root);
        projects.extend(load_config_projects(
            root, framework, &raw, &path, &source, config_dir,
        )?);
    }
    Ok(projects)
}

fn load_config_projects(
    root: &Path,
    framework: Framework,
    raw: &str,
    path: &Path,
    source: &str,
    config_dir: &Path,
) -> Result<Vec<ConfigProject>> {
    match framework {
        Framework::Playwright => {
            let parsed = test_config::playwright::parse_from_path(source, path, config_dir)?;
            Ok(parsed.into_projects(root, raw))
        }
        Framework::Vitest => {
            let parsed = test_config::vitest::parse_from_path(source, path, config_dir, root)?;
            Ok(parsed
                .into_iter()
                .map(|mut project| {
                    project.config = Some(raw.to_string());
                    project
                })
                .collect())
        }
    }
}

pub(super) fn prefix_globs(root: &Path, base: &Path, patterns: &[String]) -> Vec<String> {
    let rel = relative_slash_path(root, base);
    if rel.is_empty() || rel == "." {
        return patterns.to_vec();
    }
    patterns
        .iter()
        .map(|pattern| format!("{rel}/{pattern}"))
        .collect()
}

pub(super) fn resolve_tsconfig(root: &Path) -> Result<TsConfig> {
    if let Some(path) = find_tsconfig(root) {
        load_tsconfig(&path).context(format!("loading tsconfig {}", path.display()))
    } else {
        Ok(super::tsconfig_without_config(root))
    }
}

pub(super) fn build_globset(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
    }
    Ok(builder.build()?)
}
