use crate::analysis::routes_index::route_specificity;
use crate::analysis::selectors_index::{app_selector_targets, selector_index};
use crate::matcher;
use crate::selectors;
use std::collections::BTreeMap;
use std::path::Path;

#[test]
fn route_specificity_prefers_earlier_static_segments_and_exact_end() {
    let foo_dynamic: Vec<String> = matcher::pattern_segments("/foo/:id")
        .into_iter()
        .map(str::to_string)
        .collect();
    let dynamic_bar: Vec<String> = matcher::pattern_segments("/:section/bar")
        .into_iter()
        .map(str::to_string)
        .collect();
    let docs_exact: Vec<String> = matcher::pattern_segments("/docs")
        .into_iter()
        .map(str::to_string)
        .collect();
    let docs_catch_all: Vec<String> = matcher::pattern_segments("/docs/**")
        .into_iter()
        .map(str::to_string)
        .collect();

    assert!(route_specificity(&foo_dynamic) > route_specificity(&dynamic_bar));
    assert!(route_specificity(&docs_exact) > route_specificity(&docs_catch_all));
}

#[test]
fn selector_index_matches_exact_template_and_fuzzy_selectors() {
    let root = Path::new("/repo");
    let app_selectors = selectors::extract_app_selectors(
        Path::new("/repo/web/app/page.tsx"),
        r#"
            export function Page({ id }) {
                return <>
                    <button data-testid="save-button" />
                    <div data-testid={`user-${id}`} />
                    <span data-pw="other" />
                </>;
            }
        "#,
        &["data-testid".to_string(), "data-pw".to_string()],
        &BTreeMap::new(),
    )
    .unwrap();
    let targets = app_selector_targets(root, &app_selectors);
    let index = selector_index(&targets);

    let exact = selectors::extract_playwright_selectors(
        "await page.getByTestId('user-123');",
        &["data-testid".to_string()],
        &["data-testid".to_string()],
    );
    assert_eq!(index.matches(&exact[0]).len(), 1);

    let fuzzy = selectors::extract_playwright_selectors(
        r#"await page.locator('[data-testid^="save"]');"#,
        &["data-testid".to_string()],
        &["data-testid".to_string()],
    );
    assert_eq!(index.matches(&fuzzy[0]).len(), 1);

    let missing_value = selectors::extract_playwright_selectors(
        r#"await page.locator('[data-testid^="missing"]');"#,
        &["data-testid".to_string()],
        &["data-testid".to_string()],
    );
    assert!(index.matches(&missing_value[0]).is_empty());

    let missing_attribute = selectors::extract_playwright_selectors(
        r#"await page.locator('[data-role^="save"]');"#,
        &["data-role".to_string()],
        &["data-role".to_string()],
    );
    assert!(index.matches(&missing_attribute[0]).is_empty());
}
