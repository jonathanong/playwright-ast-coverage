use super::helpers::extract_playwright_selectors;
use crate::selectors::{AppSelector, AppSelectorValue, TemplatePattern};
use crate::test_support::fixture_source;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn attrs() -> Vec<String> {
    vec!["data-testid".to_string(), "data-pw".to_string()]
}

#[test]
fn selector_parser_handles_ast_edge_shapes() {
    let source = fixture_source(&["ast-snippets", "selectors", "edge-jsx.tsx"]);
    let selectors = crate::selectors::extract_app_selectors(
        Path::new("app/page.tsx"),
        &source,
        &attrs(),
        &BTreeMap::new(),
    )
    .unwrap();
    assert!(selectors.iter().any(|s| s.display_value() == "save"));

    let source = fixture_source(&["ast-snippets", "selectors", "edge-playwright.ts"]);
    let selectors = extract_playwright_selectors(&source, &attrs(), &["data-testid".to_string()]);
    assert!(selectors.iter().any(|s| s.selector == "getByTestId(save)"));
    assert!(selectors
        .iter()
        .any(|s| s.selector == "getByTestId(publish)"));
    assert!(selectors
        .iter()
        .any(|s| s.selector == "getByTestId(wrapped-callee)"));
    assert!(selectors
        .iter()
        .any(|s| s.selector == "getByTestId(computed-receiver)"));
    assert!(selectors
        .iter()
        .any(|s| s.selector == "getByTestId(call-receiver)"));
    assert!(selectors
        .iter()
        .any(|s| s.selector == "getByTestId(optional-receiver)"));
    assert!(selectors
        .iter()
        .any(|s| s.selector == "getByTestId(optional-call)"));
    assert!(selectors
        .iter()
        .any(|s| s.selector == r#"[data-testid="save"]"#));
}

#[test]
fn custom_test_id_attribute_maps_get_by_test_id() {
    let source = fixture_source(&["ast-snippets", "selectors", "custom-testid.ts"]);
    let selectors = extract_playwright_selectors(
        &source,
        &["data-test".to_string()],
        &["data-test".to_string()],
    );
    assert_eq!(selectors[0].attribute, "data-test");
}

#[test]
fn template_matchers_cover_structured_dynamic_values() {
    let app = AppSelector {
        file: PathBuf::from("app/page.tsx"),
        attribute: "data-testid".to_string(),
        value: AppSelectorValue::Template(TemplatePattern::new("user-${id}-button").unwrap()),
    };
    let source = fixture_source(&["ast-snippets", "selectors", "template-matchers.ts"]);
    let selectors = extract_playwright_selectors(
        &source,
        &["data-testid".to_string()],
        &["data-testid".to_string()],
    );
    assert!(!selectors.is_empty());
    assert!(selectors.iter().all(|s| app.matches_playwright(s)));
}

#[test]
fn mismatched_attributes_and_values_do_not_cover() {
    let app = AppSelector {
        file: PathBuf::from("app/page.tsx"),
        attribute: "data-testid".to_string(),
        value: AppSelectorValue::Exact("save".to_string()),
    };
    let source = fixture_source(&["ast-snippets", "selectors", "mismatched.ts"]);
    let selectors = extract_playwright_selectors(&source, &attrs(), &["data-testid".to_string()]);
    assert!(selectors.iter().all(|s| !app.matches_playwright(s)));
}

#[test]
fn unsupported_dynamic_values_never_match() {
    let source = fixture_source(&["ast-snippets", "selectors", "unsupported-dynamic.ts"]);
    let app = AppSelector {
        file: PathBuf::from("app/page.tsx"),
        attribute: "data-testid".to_string(),
        value: AppSelectorValue::Unsupported("id".to_string()),
    };
    let selectors = extract_playwright_selectors(
        &source,
        &["data-testid".to_string()],
        &["data-testid".to_string()],
    );
    assert!(!app.matches_playwright(&selectors[0]));
    assert_eq!(app.display_value(), "{id}");
}

#[test]
fn regex_flags_are_applied_to_selector_matcher() {
    let selectors = extract_playwright_selectors(
        "await page.getByTestId(/save/ims);",
        &["data-testid".to_string()],
        &["data-testid".to_string()],
    );
    assert_eq!(selectors[0].selector, "getByTestId(/save/ims)");
    let app = AppSelector {
        file: PathBuf::from("app/page.tsx"),
        attribute: "data-testid".to_string(),
        value: AppSelectorValue::Exact("SAVE".to_string()),
    };
    assert!(app.matches_playwright(&selectors[0]));
}

#[test]
fn unsupported_regex_selector_does_not_panic_or_match() {
    let app = AppSelector {
        file: PathBuf::from("app/page.tsx"),
        attribute: "data-testid".to_string(),
        value: AppSelectorValue::Exact("save".to_string()),
    };
    let selectors = extract_playwright_selectors(
        "await page.getByTestId(/(?<=prefix)save/);",
        &["data-testid".to_string()],
        &["data-testid".to_string()],
    );
    assert_eq!(selectors[0].selector, "getByTestId(/(?<=prefix)save/)");
    assert!(!app.matches_playwright(&selectors[0]));
}
