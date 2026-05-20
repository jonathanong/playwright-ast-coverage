use super::*;
use crate::selectors::SelectorMatcher;

#[test]
fn matches_handles_empty_attribute_target_bucket() {
    let mut index = SelectorIndex::default();
    index.by_attribute.insert("data-testid".into(), Vec::new());
    let selector = selectors::PlaywrightSelector::for_test(
        "data-testid",
        "button",
        SelectorMatcher::Contains("button".into()),
    );

    assert!(index.matches(&selector).is_empty());
}
