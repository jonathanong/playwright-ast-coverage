use regex::Regex;
use std::path::PathBuf;

pub struct SelectorRegexes {
    pub(super) app_attributes: Vec<String>,
    pub(super) component_attributes: std::collections::BTreeMap<String, String>,
    pub(super) playwright_attributes: Vec<AttributeRegex>,
    pub(super) html_ids: bool,
}

pub(super) struct AttributeRegex {
    pub(super) attribute: String,
    pub(super) regex: Regex,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct AppSelector {
    pub file: PathBuf,
    pub attribute: String,
    pub value: AppSelectorValue,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum AppSelectorValue {
    Exact(String),
    Template(TemplatePattern),
    Unsupported(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TemplatePattern {
    pub(super) raw: String,
    pub(super) parts: Vec<String>,
    pub(super) starts_static: bool,
    pub(super) ends_static: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PlaywrightSelector {
    pub attribute: String,
    pub selector: String,
    pub(super) matcher: SelectorMatcher,
}

impl PlaywrightSelector {
    #[cfg(test)]
    pub(crate) fn for_test(attribute: &str, selector: &str, matcher: SelectorMatcher) -> Self {
        Self {
            attribute: attribute.into(),
            selector: selector.into(),
            matcher,
        }
    }
}

#[derive(Clone, Debug)]
pub enum SelectorMatcher {
    Exact(String),
    Prefix(String),
    Suffix(String),
    Contains(String),
    Regex {
        pattern: String,
        compiled: Option<Regex>,
    },
}

impl PartialEq for SelectorMatcher {
    fn eq(&self, other: &Self) -> bool {
        self.cmp_key() == other.cmp_key()
    }
}

impl Eq for SelectorMatcher {}

impl PartialOrd for SelectorMatcher {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SelectorMatcher {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cmp_key().cmp(&other.cmp_key())
    }
}

impl SelectorMatcher {
    pub fn cmp_key(&self) -> (u8, &str) {
        match self {
            Self::Exact(value) => (0, value),
            Self::Prefix(value) => (1, value),
            Self::Suffix(value) => (2, value),
            Self::Contains(value) => (3, value),
            Self::Regex { pattern, .. } => (4, pattern),
        }
    }
}
