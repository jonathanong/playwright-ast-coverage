use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct NoMistakesConfig {
    pub filesystem: FilesystemConfig,
    pub projects: BTreeMap<String, Project>,
    pub tests: Tests,
    pub rules: HashMap<String, RuleDef>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct FilesystemConfig {
    pub skip_directories: Vec<String>,
    pub skip_file_patterns: Vec<String>,
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
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ProjectType {
    Server,
    Nextjs,
    Library,
    Tests,
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
    pub suites: Vec<TestSuitePolicy>,
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
    pub suites: Vec<TestSuitePolicy>,
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
pub struct TestSuitePolicy {
    pub name: Option<String>,
    pub config: Option<String>,
    pub project: Option<String>,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub integration: IntegrationPolicy,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum IntegrationPolicy {
    Disabled(bool),
    Suites(IntegrationSuitesPolicy),
}

impl Default for IntegrationPolicy {
    fn default() -> Self {
        Self::Disabled(false)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationSuitesPolicy {
    pub suites: Vec<String>,
    #[serde(default = "default_true")]
    pub strict: bool,
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

/// ESLint-style rule definition. The `message` and `enabled` fields are
/// reserved; all other fields are rule-specific options captured by the
/// flatten and accessible via `rule_options<T>()`.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuleDef {
    pub message: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(flatten)]
    pub options: serde_yaml::Value,
}

fn default_true() -> bool {
    true
}

impl RuleDef {
    pub fn rule_options<T: for<'de> serde::Deserialize<'de> + Default>(&self) -> T {
        serde_yaml::from_value(self.options.clone()).unwrap_or_default()
    }
}
