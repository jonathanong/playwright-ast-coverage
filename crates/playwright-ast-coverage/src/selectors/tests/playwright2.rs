use super::helpers::extract_playwright_selectors_with_regexes;
use crate::selectors::compile_selector_regexes_with_html_ids;
use std::collections::BTreeMap;
use std::path::Path;

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
