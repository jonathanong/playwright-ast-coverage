use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentFacts {
    pub name: String,
    pub file: String,
    pub environment: Environment,
    pub has_state: bool,
    pub has_props: bool,
    pub passes_props: bool,
    pub uses_memo: bool,
    pub uses_context_provider: bool,
    pub uses_suspense: bool,
    pub fetches: Vec<FetchCall>,
    pub dependencies: Vec<String>,
    pub children: Vec<ComponentRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherited_from_children: Option<AggregatedFacts>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AggregatedFacts {
    pub has_state: bool,
    pub has_props: bool,
    pub passes_props: bool,
    pub uses_memo: bool,
    pub uses_context_provider: bool,
    pub uses_suspense: bool,
    pub has_fetch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchCall {
    pub file: String,
    pub exported_name: Option<String>,
    pub shape: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentRef {
    pub name: String,
    pub file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Server,
    Client,
    Shared,
    Unknown,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Environment::Server => write!(f, "server"),
            Environment::Client => write!(f, "client"),
            Environment::Shared => write!(f, "shared"),
            Environment::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Violation {
    pub component: String,
    pub file: String,
    pub rule: String,
    pub detail: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RootConfig {
    #[serde(flatten)]
    pub legacy: FileConfig,
    pub react_traits: Option<FileConfig>,
}

#[derive(Default, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct FileConfig {
    pub frontend_root: Option<String>,
    pub assert_no_fetch: Option<bool>,
}
