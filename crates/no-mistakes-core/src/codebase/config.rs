use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
    let Some(path) = find_config_file(start) else {
        let mut config = Config::default();
        config.augment_from_gitignore(start);
        return Ok(config);
    };

    let yaml =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut config =
        Config::from_yaml(&yaml).with_context(|| format!("parsing {}", path.display()))?;
    config.augment_from_gitignore(path.parent().unwrap_or(start));
    Ok(config)
}

fn find_config_file(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join(".guardrailsrc.yml");
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}
