use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::config::resolve;
use crate::config::v2::{find_config_root, load_v2_config, schema::NoMistakesConfig};
use crate::config::CONFIG_EXTENSIONS;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FilesystemConfig {
    #[serde(default)]
    pub skip_directories: Vec<String>,
    #[serde(default)]
    pub skip_file_patterns: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct RouteOptions {
    pub backend_pattern: String,
    pub backend_register_object: String,
    pub frontend_root: String,
    pub scan_patterns: Vec<String>,
    pub backend_prefixes: Vec<String>,
    pub backend_exact_paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct QueueOptions {
    pub queue_pattern: String,
    pub factory_specifier: String,
    pub factory_function: String,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct HttpRouteOptions {
    pub backend_pattern: String,
    pub register_object: String,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct HttpCallOptions {
    pub backend_prefixes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
pub struct RuleConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(flatten, default)]
    pub options: serde_yaml::Value,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub filesystem: FilesystemConfig,
    #[serde(default)]
    pub rules: HashMap<String, RuleConfig>,
}

impl Config {
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml).context("failed to parse .guardrailsrc.yml")
    }

    pub fn rule_options<T: for<'de> Deserialize<'de> + Default>(&self, rule_id: &str) -> T {
        self.rules
            .get(rule_id)
            .and_then(|rule| serde_yaml::from_value(rule.options.clone()).ok())
            .unwrap_or_default()
    }

    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
        self.rules
            .get(rule_id)
            .map(|rule| rule.enabled)
            .unwrap_or(true)
    }

    pub fn augment_from_gitignore(&mut self, root: &Path) {
        let Ok(content) = std::fs::read_to_string(root.join(".gitignore")) else {
            return;
        };

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.starts_with('#')
                || trimmed.starts_with('!')
                || trimmed.contains('/')
                || trimmed.contains('*')
                || trimmed.contains('?')
                || trimmed.contains('[')
            {
                continue;
            }

            let directory = trimmed.to_string();
            if !self.filesystem.skip_directories.contains(&directory) {
                self.filesystem.skip_directories.push(directory);
            }
        }
    }
}

pub fn load_config(start: &Path) -> Result<Config> {
    load_config_with_path(start, None)
}

pub fn load_config_with_path(start: &Path, config_path: Option<&Path>) -> Result<Config> {
    let v2 = load_v2_config(start, config_path)?;
    let mut config = config_from_v2(v2);
    let gitignore_root = match config_path {
        Some(path) => {
            let resolved = resolve(start, path);
            resolved
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| start.to_path_buf())
        }
        None => find_config_root(start),
    };
    config.augment_from_gitignore(&gitignore_root);
    Ok(config)
}

pub fn load_codebase_config_with_path(start: &Path, config_path: Option<&Path>) -> Result<Config> {
    if config_path.is_some() {
        return load_config_with_path(start, config_path);
    }

    let Some(path) = find_codebase_config_path(start)? else {
        let mut config = Config::default();
        config.augment_from_gitignore(start);
        return Ok(config);
    };

    let v2 = load_v2_config(start, Some(&path))?;
    let mut config = config_from_v2(v2);
    config.augment_from_gitignore(path.parent().unwrap_or(start));
    Ok(config)
}

fn find_codebase_config_path(start: &Path) -> Result<Option<std::path::PathBuf>> {
    let mut current = start.to_path_buf();
    loop {
        if let Some(path) = find_config_for_stem(&current, ".no-mistakes")? {
            return Ok(Some(path));
        }
        if let Some(path) = find_config_for_stem(&current, ".guardrailsrc")? {
            return Ok(Some(path));
        }
        if !current.pop() {
            return Ok(None);
        }
    }
}

fn find_config_for_stem(root: &Path, stem: &str) -> Result<Option<std::path::PathBuf>> {
    let found = CONFIG_EXTENSIONS
        .iter()
        .map(|ext| root.join(format!("{stem}.{ext}")))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    match found.len() {
        0 => Ok(None),
        1 => Ok(found.into_iter().next()),
        _ => {
            let files = found
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::bail!("multiple config files found under --root: {files}");
        }
    }
}

fn config_from_v2(v2: NoMistakesConfig) -> Config {
    let rules = v2
        .rules
        .into_iter()
        .map(|(id, def)| {
            (
                id,
                RuleConfig {
                    enabled: def.enabled,
                    options: def.options,
                },
            )
        })
        .collect();
    Config {
        filesystem: FilesystemConfig {
            skip_directories: v2.filesystem.skip_directories,
            skip_file_patterns: v2.filesystem.skip_file_patterns,
        },
        rules,
    }
}

#[cfg(test)]
mod tests;
