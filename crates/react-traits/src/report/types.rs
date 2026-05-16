use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ComponentFacts {
    pub(crate) name: String,
    pub(crate) file: String,
    pub(crate) environment: Environment,
    pub(crate) has_state: bool,
    pub(crate) has_props: bool,
    pub(crate) passes_props: bool,
    pub(crate) uses_memo: bool,
    pub(crate) uses_context_provider: bool,
    pub(crate) uses_suspense: bool,
    pub(crate) fetches: Vec<FetchCall>,
    pub(crate) dependencies: Vec<String>,
    pub(crate) children: Vec<ComponentRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) inherited_from_children: Option<AggregatedFacts>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AggregatedFacts {
    pub(crate) has_state: bool,
    pub(crate) has_props: bool,
    pub(crate) passes_props: bool,
    pub(crate) uses_memo: bool,
    pub(crate) uses_context_provider: bool,
    pub(crate) uses_suspense: bool,
    pub(crate) has_fetch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FetchCall {
    pub(crate) file: String,
    pub(crate) exported_name: Option<String>,
    pub(crate) shape: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ComponentRef {
    pub(crate) name: String,
    pub(crate) file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Environment {
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
pub(crate) struct Violation {
    pub(crate) component: String,
    pub(crate) file: String,
    pub(crate) rule: String,
    pub(crate) detail: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct RootConfig {
    #[serde(flatten)]
    pub(crate) legacy: FileConfig,
    pub(crate) react_traits: Option<FileConfig>,
}

#[derive(Default, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct FileConfig {
    pub(crate) frontend_root: Option<String>,
    pub(crate) assert_no_fetch: Option<bool>,
}
