use crate::selectors::types::SelectorMatcher;

#[test]
fn playwright_selector_order_uses_matcher_kind_and_pattern() {
    let mut matchers = [
        SelectorMatcher::Contains("v".to_string()),
        SelectorMatcher::Regex {
            pattern: "^v".to_string(),
            compiled: regex::Regex::new("^v").ok(),
        },
        SelectorMatcher::Suffix("v".to_string()),
        SelectorMatcher::Prefix("v".to_string()),
        SelectorMatcher::Exact("v".to_string()),
    ];

    assert_eq!(matchers[0], matchers[0]);
    matchers.sort();
    assert_eq!(
        matchers
            .iter()
            .map(SelectorMatcher::cmp_key)
            .collect::<Vec<_>>(),
        vec![(0, "v"), (1, "v"), (2, "v"), (3, "v"), (4, "^v")]
    );
}
