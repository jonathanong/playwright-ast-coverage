use crate::ast;
use crate::playwright_tests::TestStatus;
use crate::selectors::{
    compile_selector_regexes, extract_playwright_selector_occurrences_from_program,
    PlaywrightSelector, SelectorRegexes,
};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn extract_playwright_selectors(
    source: &str,
    selector_attributes: &[String],
    test_id_attributes: &[String],
) -> Vec<PlaywrightSelector> {
    let regexes = compile_selector_regexes(selector_attributes, &BTreeMap::new());
    extract_playwright_selectors_with_regexes(
        Path::new("fixture.ts"),
        source,
        &regexes,
        test_id_attributes,
    )
    .expect("fixture should parse")
}

pub(super) fn extract_playwright_selectors_with_regexes(
    path: &Path,
    source: &str,
    regexes: &SelectorRegexes,
    test_id_attributes: &[String],
) -> anyhow::Result<Vec<PlaywrightSelector>> {
    ast::with_program(path, source, |program, source| {
        extract_playwright_selector_occurrences_from_program(
            program,
            source,
            regexes,
            test_id_attributes,
        )
        .into_iter()
        .map(|o| o.value)
        .collect()
    })
}

pub(super) fn extract_playwright_selector_occurrences(
    source: &str,
    selector_attributes: &[String],
    test_id_attributes: &[String],
) -> Vec<(String, TestStatus)> {
    let regexes = compile_selector_regexes(selector_attributes, &BTreeMap::new());
    ast::with_program(Path::new("fixture.ts"), source, |program, source| {
        extract_playwright_selector_occurrences_from_program(
            program,
            source,
            &regexes,
            test_id_attributes,
        )
        .into_iter()
        .map(|o| (o.value.selector, o.status))
        .collect()
    })
    .expect("fixture should parse")
}
