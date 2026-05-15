use super::types::{
    AppSelector, AppSelectorValue, PlaywrightSelector, SelectorMatcher, TemplatePattern,
};

impl SelectorMatcher {
    pub(crate) fn matches_value(&self, value: &str) -> bool {
        match self {
            Self::Exact(expected) => value == expected,
            Self::Prefix(prefix) => value.starts_with(prefix),
            Self::Suffix(suffix) => value.ends_with(suffix),
            Self::Contains(part) => value.contains(part),
            Self::Regex { compiled, .. } => {
                compiled.as_ref().is_some_and(|regex| regex.is_match(value))
            }
        }
    }

    pub(crate) fn matches_pattern(&self, pattern: &TemplatePattern) -> bool {
        match self {
            Self::Exact(value) => pattern.matches_exact(value),
            Self::Prefix(prefix) => pattern
                .first_static()
                .is_some_and(|part| part.starts_with(prefix) || prefix.starts_with(part)),
            Self::Suffix(suffix) => pattern
                .last_static()
                .is_some_and(|part| part.ends_with(suffix) || suffix.ends_with(part)),
            Self::Contains(part) => pattern.contains_static(part),
            Self::Regex { compiled, .. } => compiled
                .as_ref()
                .is_some_and(|regex| regex.is_match(&pattern.sample())),
        }
    }
}

impl AppSelectorValue {
    pub fn display_value(&self) -> String {
        match self {
            Self::Exact(value) => value.clone(),
            Self::Template(pattern) => pattern.raw.clone(),
            Self::Unsupported(value) => format!("{{{value}}}"),
        }
    }

    pub fn matches_selector(&self, matcher: &SelectorMatcher) -> bool {
        match self {
            Self::Exact(value) => matcher.matches_value(value),
            Self::Template(pattern) => matcher.matches_pattern(pattern),
            Self::Unsupported(_) => false,
        }
    }
}

impl AppSelector {
    pub fn display_value(&self) -> String {
        self.value.display_value()
    }

    pub fn unsupported_dynamic(&self) -> bool {
        matches!(self.value, AppSelectorValue::Unsupported(_))
    }

    pub fn matches_playwright(&self, selector: &PlaywrightSelector) -> bool {
        self.attribute == selector.attribute && self.value.matches_selector(&selector.matcher)
    }
}

impl PlaywrightSelector {
    pub fn exact_value(&self) -> Option<&str> {
        match &self.matcher {
            SelectorMatcher::Exact(value) => Some(value),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
