use crate::analysis::context::{AppSelectorTarget, SelectorIndex};
use crate::fsutil::relative_string;
use crate::selectors::{self, AppSelectorValue};
use std::path::Path;

pub(crate) fn app_selector_targets<'a>(
    root: &Path,
    app_selectors: &'a [selectors::AppSelector],
) -> Vec<AppSelectorTarget<'a>> {
    app_selectors
        .iter()
        .map(|selector| AppSelectorTarget {
            selector,
            app_file: relative_string(root, &selector.file),
            value: selector.display_value(),
        })
        .collect()
}

pub(crate) fn selector_index<'a>(targets: &'a [AppSelectorTarget<'a>]) -> SelectorIndex<'a> {
    let mut index = SelectorIndex::default();
    for target in targets {
        if target.selector.unsupported_dynamic() {
            continue;
        }
        index
            .by_attribute
            .entry(target.selector.attribute.clone())
            .or_default()
            .push(target);
        if let AppSelectorValue::Exact(value) = &target.selector.value {
            index
                .exact
                .entry(target.selector.attribute.clone())
                .or_default()
                .entry(value.clone())
                .or_default()
                .push(target);
        }
        if matches!(target.selector.value, AppSelectorValue::Template(_)) {
            index
                .templates_by_attribute
                .entry(target.selector.attribute.clone())
                .or_default()
                .push(target);
        }
    }
    index
}

impl<'a> SelectorIndex<'a> {
    pub(crate) fn matches(
        &'a self,
        playwright_selector: &selectors::PlaywrightSelector,
    ) -> Vec<&'a AppSelectorTarget<'a>> {
        let mut matches = Vec::new();
        if let Some(value) = playwright_selector.exact_value() {
            if let Some(by_value) = self.exact.get(&playwright_selector.attribute) {
                if let Some(exact) = by_value.get(value) {
                    matches.extend(exact.iter().copied());
                }
            }
            let Some(attribute_targets) = self
                .templates_by_attribute
                .get(&playwright_selector.attribute)
            else {
                return matches;
            };
            for target in attribute_targets {
                if target.selector.matches_playwright(playwright_selector) {
                    matches.push(*target);
                }
            }
            return matches;
        }

        if let Some(attribute_targets) = self.by_attribute.get(&playwright_selector.attribute) {
            for target in attribute_targets {
                if target.selector.matches_playwright(playwright_selector) {
                    matches.push(*target);
                }
            }
        }
        matches
    }
}
