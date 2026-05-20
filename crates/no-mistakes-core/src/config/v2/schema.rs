use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct NoMistakesConfig {
    pub filesystem: FilesystemConfig,
    pub projects: BTreeMap<String, Project>,
    pub tests: Tests,
    pub rules: Vec<RuleDef>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct FilesystemConfig {
    pub skip_directories: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Project {
    #[serde(rename = "type")]
    pub type_: Option<ProjectType>,
    pub root: Option<String>,
    pub include: Vec<String>,
    pub routes: Vec<String>,
    pub queues: QueueConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectType {
    Server,
    Nextjs,
    Library,
    Tests,
    Rust,
    CloudflareWorkers,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct QueueConfig {
    pub enqueues: Vec<String>,
    pub workers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Tests {
    pub playwright: PlaywrightTestConfig,
    pub vitest: VitestConfig,
    pub jest: JestConfig,
    pub storybook: StorybookConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct PlaywrightTestConfig {
    pub configs: Option<StringOrList>,
    pub projects: BTreeMap<String, TestProjectPolicy>,
    pub selectors: PlaywrightSelectors,
    pub selector_roots: Vec<String>,
    pub selector_exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct PlaywrightSelectors {
    pub html_ids: bool,
    pub test_ids: Vec<String>,
    pub component_test_ids: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct VitestConfig {
    pub configs: Option<StringOrList>,
    pub projects: BTreeMap<String, TestProjectPolicy>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct JestConfig {
    pub configs: Option<StringOrList>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct StorybookConfig {
    pub configs: Option<StringOrList>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct TestProjectPolicy {
    #[serde(rename = "integration_suites")]
    pub integration_suites: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum StringOrList {
    One(String),
    Many(Vec<String>),
}

impl StringOrList {
    pub fn values(&self) -> Vec<String> {
        match self {
            Self::One(s) => vec![s.clone()],
            Self::Many(v) => v.clone(),
        }
    }
}

/// A configured rule application.
///
/// Unlike ESLint-style rule maps, no-mistakes rules are reusable applications:
/// the same `rule` can be attached to different projects or test groups with
/// different names and options.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RuleDef {
    pub name: Option<String>,
    pub rule: String,
    pub message: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub projects: Vec<String>,
    pub tests: RuleTestTargets,
    pub scope: Option<RuleScope>,
    #[serde(default = "empty_options")]
    pub options: serde_yaml::Value,
}

impl Default for RuleDef {
    fn default() -> Self {
        Self {
            name: None,
            rule: String::new(),
            message: None,
            enabled: true,
            projects: Vec::new(),
            tests: RuleTestTargets::default(),
            scope: None,
            options: empty_options(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RuleTestTargets {
    pub vitest: Vec<String>,
    pub playwright: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum RuleScope {
    Repository,
}

fn default_true() -> bool {
    true
}

fn empty_options() -> serde_yaml::Value {
    serde_yaml::Value::Mapping(Default::default())
}

impl RuleDef {
    pub fn rule_options<T: for<'de> serde::Deserialize<'de> + Default>(&self) -> T {
        serde_yaml::from_value(self.options.clone()).unwrap_or_default()
    }

    pub fn applies_to_project(&self, project: &str) -> bool {
        self.enabled && self.projects.iter().any(|name| name == project)
    }

    pub fn applies_to_repository(&self) -> bool {
        self.enabled && self.scope == Some(RuleScope::Repository)
    }
}

impl NoMistakesConfig {
    pub fn rule_applications<'a>(&'a self, rule_id: &str) -> Vec<&'a RuleDef> {
        self.rules
            .iter()
            .filter(move |rule| rule.enabled && rule.rule == rule_id)
            .collect()
    }

    pub fn rule_configured(&self, rule_id: &str) -> bool {
        !self.rule_applications(rule_id).is_empty()
    }

    pub fn rule_options<T: for<'de> serde::Deserialize<'de> + Default>(&self, rule_id: &str) -> T {
        self.rule_applications(rule_id)
            .into_iter()
            .next()
            .map(RuleDef::rule_options)
            .unwrap_or_default()
    }
}
