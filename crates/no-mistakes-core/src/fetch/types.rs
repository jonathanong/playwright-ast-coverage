use serde::Serialize;

#[derive(Serialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct FetchOccurrence {
    pub path: String,
    pub raw_path: String,
    pub method: String,
    pub file: String,
    pub line: usize,
    pub side: FetchSide,
    #[serde(rename = "rsc")]
    pub rsc: bool,
    pub cached: bool,
    pub cache_kind: CacheKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_function: Option<String>,
    pub dynamic: bool,
    pub unsupported: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub struct UrlExtraction {
    pub path: String,
    pub raw_path: String,
    pub is_dynamic: bool,
    pub is_unsupported: bool,
}

#[derive(Serialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum FetchSide {
    Client,
    Server,
}

#[derive(Serialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "kebab-case")]
#[allow(dead_code)]
pub enum CacheKind {
    None,
    FetchCache,
    FetchNextRevalidate,
    FetchNextTags,
    ReactCache,
    Cache,
    UnstableCache,
}
