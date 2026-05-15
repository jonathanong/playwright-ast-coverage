use crate::selectors::{
    collect_app_selectors, compile_selector_regexes, compile_selector_regexes_with_html_ids,
    extract_app_selectors, extract_app_selectors_with_regexes, AppSelector, AppSelectorValue,
    SelectorMatcher,
};
use crate::test_support::{fixture_path, fixture_source};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn attrs() -> Vec<String> {
    vec!["data-testid".to_string(), "data-pw".to_string()]
}

fn component_attrs() -> BTreeMap<String, String> {
    BTreeMap::new()
}

#[test]
fn extracts_static_jsx_selectors() {
    let source = fixture_source(&["ast-snippets", "selectors", "static-jsx.tsx"]);
    let selectors = extract_app_selectors(
        Path::new("app/page.tsx"),
        &source,
        &attrs(),
        &component_attrs(),
    )
    .unwrap();
    let mut values: Vec<String> = selectors.iter().map(AppSelector::display_value).collect();
    values.sort();
    assert_eq!(values, vec!["delete", "publish", "save"]);
}

#[test]
fn extracts_template_and_unsupported_jsx_selectors() {
    let source = fixture_source(&["ast-snippets", "selectors", "template-and-unsupported.tsx"]);
    let selectors = extract_app_selectors(
        Path::new("app/page.tsx"),
        &source,
        &attrs(),
        &component_attrs(),
    )
    .unwrap();
    assert!(selectors
        .iter()
        .any(|selector| selector.display_value() == "user-${id}"));
    assert!(selectors.iter().any(AppSelector::unsupported_dynamic));
}

#[test]
fn maps_component_selector_attributes_to_dom_attributes() {
    let mut component_attributes = BTreeMap::new();
    component_attributes.insert("dataPw".to_string(), "data-pw".to_string());
    let selectors = extract_app_selectors(
        Path::new("app/page.tsx"),
        r#"
        export function Page() {
            return <>
                <SaveButton dataPw="save" />
                <_SaveButton dataPw="private" />
                <$SaveButton dataPw="dollar" />
                <UI.Button dataPw="publish" />
                <button dataPw="ignored" />
                <custom-element dataPw="ignored-custom" />
                <SaveButton data-pw="legacy" />
                <SaveButton {...props} />
            </>;
        }
        "#,
        &attrs(),
        &component_attributes,
    )
    .unwrap();

    let values: BTreeSet<(String, String)> = selectors
        .iter()
        .map(|selector| (selector.attribute.clone(), selector.display_value()))
        .collect();
    assert_eq!(
        values,
        BTreeSet::from([
            ("data-pw".to_string(), "legacy".to_string()),
            ("data-pw".to_string(), "dollar".to_string()),
            ("data-pw".to_string(), "publish".to_string()),
            ("data-pw".to_string(), "private".to_string()),
            ("data-pw".to_string(), "save".to_string()),
        ])
    );
}

#[test]
fn collect_app_selectors_reads_source_files_and_skips_build_dirs() {
    let root = fixture_path(&["ast-snippets", "selectors", "collect-app"]);
    let selectors = collect_app_selectors(&root, &attrs()).unwrap();
    assert_eq!(selectors.len(), 1);
    assert_eq!(selectors[0].display_value(), "ok");
    assert!(collect_app_selectors(&root.join("missing"), &attrs())
        .unwrap()
        .is_empty());
    let invalid = fixture_path(&[
        "ast-snippets",
        "main",
        "invalid-selector-source",
        "web",
        "app",
    ]);
    assert!(collect_app_selectors(&invalid, &attrs()).is_err());
}

#[test]
fn extracts_html_ids_when_enabled() {
    let regexes = compile_selector_regexes_with_html_ids(
        &["data-testid".to_string()],
        &BTreeMap::new(),
        true,
    );
    let app_selectors = extract_app_selectors_with_regexes(
        Path::new("app/page.tsx"),
        r#"
        export function Page({ id }) {
            return <>
                <button id="save" />
                <button id={`user-${id}`} />
                <button data-testid="publish" />
                <CustomWidget id="internal" />
            </>;
        }
        "#,
        &regexes,
    )
    .unwrap();
    let values: BTreeSet<(String, String)> = app_selectors
        .iter()
        .map(|s| (s.attribute.clone(), s.display_value()))
        .collect();
    assert_eq!(
        values,
        BTreeSet::from([
            ("data-testid".to_string(), "publish".to_string()),
            ("id".to_string(), "save".to_string()),
            ("id".to_string(), "user-${id}".to_string()),
        ])
    );
}

#[test]
fn exact_and_operator_matchers_cover_static_values() {
    let app = AppSelector {
        file: PathBuf::from("app/page.tsx"),
        attribute: "data-testid".to_string(),
        value: AppSelectorValue::Exact("save-button".to_string()),
    };
    let source = fixture_source(&["ast-snippets", "selectors", "exact-operator-matchers.ts"]);
    let selectors = crate::selectors::extract_playwright_selectors(
        &source,
        &["data-testid".to_string()],
        &["data-testid".to_string()],
    );
    assert!(selectors.iter().all(|s| app.matches_playwright(s)));

    assert!(!AppSelectorValue::Unsupported("x".to_string())
        .matches_selector(&SelectorMatcher::Exact("x".to_string())));

    let mut regexes = BTreeMap::new();
    regexes.insert("dataPw".to_string(), "data-pw".to_string());
    let selector_regexes = compile_selector_regexes(&["data-testid".to_string()], &regexes);
    assert!(selector_regexes
        .app_attributes
        .contains(&"data-testid".to_string()));
}

#[test]
fn component_jsx_name_checks() {
    use super::helpers::extract_playwright_selectors_with_regexes;
    let source = "const x = <ns:name />; const y = <this />;";
    let regexes = compile_selector_regexes(&["data-testid".to_string()], &BTreeMap::new());
    let selectors = extract_playwright_selectors_with_regexes(
        Path::new("fixture.tsx"),
        source,
        &regexes,
        &["data-testid".to_string()],
    )
    .unwrap();
    assert!(selectors.is_empty());
}
