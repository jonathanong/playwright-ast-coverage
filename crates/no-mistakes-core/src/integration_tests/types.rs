use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationFinding {
    pub framework: String,
    pub suite: String,
    pub file: String,
    pub line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub describe_path: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Framework {
    Playwright,
    Vitest,
}

impl Framework {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Playwright => "playwright",
            Self::Vitest => "vitest",
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct Suite {
    pub framework: Framework,
    pub name: String,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub policy: EffectiveIntegrationPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum EffectiveIntegrationPolicy {
    Disabled,
    Suites { suites: Vec<String>, strict: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TestCase {
    pub name: Option<String>,
    pub describe_path: Vec<String>,
    pub function_key: FunctionKey,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct FunctionKey {
    pub file: PathBuf,
    pub name: String,
}

#[derive(Debug, Clone)]
pub(super) struct FunctionInfo {
    pub integration: Option<String>,
    pub calls: Vec<CallTarget>,
}

#[derive(Debug, Clone)]
pub(super) enum CallTarget {
    Local(String),
    Imported { local: String },
    Namespace { namespace: String, member: String },
}

#[derive(Debug, Clone)]
pub(super) struct ImportBinding {
    pub source: String,
    pub imported: ImportedName,
}

#[derive(Debug, Clone)]
pub(super) enum ImportedName {
    Named(String),
    Default,
    Namespace,
}

#[derive(Clone, Default)]
pub(crate) struct FileAnalysis {
    pub(super) imports: HashMap<String, ImportBinding>,
    pub(super) exports: HashMap<String, String>,
    pub(super) functions: HashMap<String, FunctionInfo>,
    pub(super) tests: Vec<TestCase>,
}

#[derive(Debug, Clone)]
pub(super) struct ConfigProject {
    pub config: Option<String>,
    pub name: Option<String>,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}
