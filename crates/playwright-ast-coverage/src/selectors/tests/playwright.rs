use super::helpers::{
    extract_playwright_selector_occurrences, extract_playwright_selectors,
    extract_playwright_selectors_with_regexes,
};
use crate::playwright_tests::TestStatus;
use crate::selectors::compile_selector_regexes_with_html_ids;
use std::collections::BTreeMap;
use std::path::Path;

fn attrs() -> Vec<String> {
    vec!["data-testid".to_string(), "data-pw".to_string()]
}

#[test]
fn extracts_playwright_css_and_test_id_selectors() {
    let source =
        crate::test_support::fixture_source(&["selectors", "playwright-css-and-testid.ts"]);
    let selectors = extract_playwright_selectors(&source, &attrs(), &["data-testid".to_string()]);
    assert!(selectors.iter().any(|s| s.selector == "getByTestId(save)"));
    assert!(selectors
        .iter()
        .any(|s| s.selector == "[data-testid^='user-']"));
    assert!(selectors
        .iter()
        .any(|s| s.selector == r#"[data-pw$="button"]"#));
    assert!(selectors
        .iter()
        .any(|s| s.selector == r#"[data-pw*="nav"]"#));
    assert!(selectors
        .iter()
        .any(|s| s.selector == r#"[data-pw="exact"]"#));
    assert!(selectors
        .iter()
        .any(|s| s.selector == "getByTestId(/^account-/)"));
}

#[test]
fn marks_selectors_inside_skipped_and_conditional_tests() {
    let selectors = extract_playwright_selector_occurrences(
        r#"
        test.skip('skipped', async ({ page }) => { await page.getByTestId('skipped'); });
        test.fixme('fixme test', async ({ page }) => { await page.getByTestId('fixme'); });
        if (process.env.E2E) {
            test('conditional wrapper', async ({ page }) => {
                await page.getByTestId('conditional-wrapper');
            });
        } else {
            test('conditional alternate', async ({ page }) => {
                await page.locator('[data-testid="conditional-alternate"]');
            });
        }
        featureFlag && test('logical wrapper', async ({ page }) => {
            await page.getByTestId('logical-wrapper');
        });
        featureFlag
            ? test('ternary consequent', async ({ page }) => {
                await page.getByTestId('ternary-consequent');
            })
            : test('ternary alternate', async ({ page }) => {
                await page.getByTestId('ternary-alternate');
            });
        test('active', async ({ page }) => { await page.getByTestId('active'); });
        test.skip(({ browserName }) => browserName === 'webkit', 'conditional');
        test('file scope annotation', async ({ page }) => {
            await page.getByTestId('scope-annotation');
        });
        "#,
        &attrs(),
        &["data-testid".to_string()],
    );

    assert_eq!(
        selectors,
        vec![
            (
                r#"[data-testid="conditional-alternate"]"#.to_string(),
                TestStatus::Conditional
            ),
            ("getByTestId(active)".to_string(), TestStatus::Active),
            (
                "getByTestId(conditional-wrapper)".to_string(),
                TestStatus::Conditional
            ),
            ("getByTestId(fixme)".to_string(), TestStatus::Skipped),
            (
                "getByTestId(logical-wrapper)".to_string(),
                TestStatus::Conditional
            ),
            (
                "getByTestId(scope-annotation)".to_string(),
                TestStatus::Conditional
            ),
            ("getByTestId(skipped)".to_string(), TestStatus::Skipped),
            (
                "getByTestId(ternary-alternate)".to_string(),
                TestStatus::Conditional
            ),
            (
                "getByTestId(ternary-consequent)".to_string(),
                TestStatus::Conditional
            ),
        ]
    );
}

#[test]
fn css_attribute_selectors_must_be_used_by_playwright_selector_calls() {
    let source = r#"
        const unused = '[data-testid="save"]';
        await page.locator('[data-testid="publish"]').click();
        await page.click(`[data-pw="open"]`);
        await page.type('[data-testid="search"]', 'query');
        await page.$eval('[data-pw="panel"]', node => node.textContent);
        await page.$$eval('[data-testid="items"]', nodes => nodes.length);
        await page.frameLocator('[data-pw="frame"]').locator('[data-testid="inside"]');
        await page.dragAndDrop('[data-testid="source"]', '[data-pw="target"]');
    "#;
    let selectors = extract_playwright_selectors(source, &attrs(), &["data-testid".to_string()]);
    assert!(selectors
        .iter()
        .any(|s| s.selector == r#"[data-testid="publish"]"#));
    assert!(selectors
        .iter()
        .any(|s| s.selector == r#"[data-pw="open"]"#));
    assert!(selectors
        .iter()
        .any(|s| s.selector == r#"[data-testid="search"]"#));
    assert!(selectors
        .iter()
        .any(|s| s.selector == r#"[data-pw="panel"]"#));
    assert!(selectors
        .iter()
        .any(|s| s.selector == r#"[data-testid="items"]"#));
    assert!(selectors
        .iter()
        .any(|s| s.selector == r#"[data-pw="frame"]"#));
    assert!(selectors
        .iter()
        .any(|s| s.selector == r#"[data-testid="inside"]"#));
    assert!(selectors
        .iter()
        .any(|s| s.selector == r#"[data-testid="source"]"#));
    assert!(selectors
        .iter()
        .any(|s| s.selector == r#"[data-pw="target"]"#));
    assert!(selectors
        .iter()
        .all(|s| s.selector != r#"[data-testid="save"]"#));
}

#[test]
fn extracts_html_ids_playwright_selectors() {
    let regexes = compile_selector_regexes_with_html_ids(
        &["data-testid".to_string()],
        &BTreeMap::new(),
        true,
    );
    let playwright_selectors = extract_playwright_selectors_with_regexes(
        Path::new("tests/app.spec.ts"),
        r#"
        await page.locator('#save').click();
        await page.locator('button#user-42 .label').click();
        await page.locator('#save, #publish').click();
        await page.locator('[id="save"]').click();
        "#,
        &regexes,
        &["data-testid".to_string()],
    )
    .unwrap();
    let values: std::collections::BTreeSet<(String, String)> = playwright_selectors
        .iter()
        .map(|s| (s.attribute.clone(), s.selector.clone()))
        .collect();
    assert_eq!(
        values,
        std::collections::BTreeSet::from([
            ("id".to_string(), "#publish".to_string()),
            ("id".to_string(), "#save".to_string()),
            ("id".to_string(), "#user-42".to_string()),
            ("id".to_string(), r#"[id="save"]"#.to_string()),
        ])
    );
}

#[test]
fn css_id_selectors_ignore_hashes_inside_attribute_values_and_decode_escapes() {
    let regexes = compile_selector_regexes_with_html_ids(&[], &BTreeMap::new(), true);
    let selectors = extract_playwright_selectors_with_regexes(
        Path::new("tests/app.spec.ts"),
        r##"
        await page.locator('a[href="#save"]').click();
        await page.locator('#save\\:button').click();
        await page.locator('#escaped\\20 space').click();
        "##,
        &regexes,
        &["data-testid".to_string()],
    )
    .unwrap();
    let values: Vec<(&str, Option<&str>)> = selectors
        .iter()
        .map(|s| (s.selector.as_str(), s.exact_value()))
        .collect();
    assert_eq!(
        values,
        vec![
            ("#escaped\\20 space", Some("escaped space")),
            ("#save\\:button", Some("save:button")),
        ]
    );
}

#[test]
fn css_id_selectors_handle_escaped_hashes_quotes_empty_ids_and_hex_whitespace() {
    let regexes = compile_selector_regexes_with_html_ids(&[], &BTreeMap::new(), true);
    let selectors = extract_playwright_selectors_with_regexes(
        Path::new("tests/app.spec.ts"),
        r##"
        await page.locator('.literal\\#hash #real').click();
        await page.locator("a[data-label='quoted #ignored'] #quoted").click();
        await page.locator('/* #deprecated */ #commented').click();
        await page.locator('# .empty #after-empty').click();
        await page.locator('#hex\\2dnext').click();
        await page.locator('#six\\000020 spaced').click();
        await page.locator(`#user-${id}`).click();
        "##,
        &regexes,
        &["data-testid".to_string()],
    )
    .unwrap();
    let values: Vec<(&str, Option<&str>)> = selectors
        .iter()
        .map(|s| (s.selector.as_str(), s.exact_value()))
        .collect();
    assert_eq!(
        values,
        vec![
            ("#after-empty", Some("after-empty")),
            ("#commented", Some("commented")),
            ("#hex\\2dnext", Some("hex-next")),
            ("#quoted", Some("quoted")),
            ("#real", Some("real")),
            ("#six\\000020 spaced", Some("six spaced")),
        ]
    );
}
