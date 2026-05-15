use crate::analysis::types::{DuplicateSelector, UniqueSelectorPolicy};
use crate::fsutil::relative_string;
use crate::selectors::{self, AppSelectorValue};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum DuplicateSelectorKey<'a> {
    Aggregate(&'a str),
    TestId(&'a str),
    HtmlId(&'a str),
}

impl DuplicateSelectorKey<'_> {
    pub(crate) fn value(&self) -> &str {
        match self {
            Self::Aggregate(value) | Self::TestId(value) | Self::HtmlId(value) => value,
        }
    }
}

pub(crate) fn build_duplicate_selectors(
    root: &Path,
    app_selectors: &[selectors::AppSelector],
    policy: UniqueSelectorPolicy,
) -> Vec<DuplicateSelector> {
    let mut by_value: BTreeMap<DuplicateSelectorKey<'_>, Vec<&selectors::AppSelector>> =
        BTreeMap::new();
    for selector in app_selectors {
        if let AppSelectorValue::Exact(value) = &selector.value {
            if policy.aggregate {
                by_value
                    .entry(DuplicateSelectorKey::Aggregate(value.as_str()))
                    .or_default()
                    .push(selector);
            } else if selector.attribute == selectors::HTML_ID_ATTRIBUTE {
                if policy.html_ids || (policy.test_ids && policy.configured_html_id_selector) {
                    by_value
                        .entry(DuplicateSelectorKey::HtmlId(value.as_str()))
                        .or_default()
                        .push(selector);
                }
            } else if policy.test_ids {
                by_value
                    .entry(DuplicateSelectorKey::TestId(value.as_str()))
                    .or_default()
                    .push(selector);
            }
        }
    }

    let mut duplicates = Vec::new();
    for (key, selectors) in by_value {
        if selectors.len() < 2 {
            continue;
        }
        let value = key.value().to_string();
        for selector in selectors {
            duplicates.push(DuplicateSelector {
                attribute: selector.attribute.clone(),
                value: value.clone(),
                file: relative_string(root, &selector.file),
            });
        }
    }
    duplicates.sort_by(|a, b| {
        a.value
            .cmp(&b.value)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.attribute.cmp(&b.attribute))
    });
    duplicates
}
