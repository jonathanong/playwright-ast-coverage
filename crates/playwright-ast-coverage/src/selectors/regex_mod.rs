use super::types::{AttributeRegex, SelectorMatcher, SelectorRegexes};
use super::HTML_ID_ATTRIBUTE;
use regex::Regex;
use std::collections::BTreeMap;

#[cfg(test)]
pub fn compile_selector_regexes(
    attributes: &[String],
    component_attributes: &BTreeMap<String, String>,
) -> SelectorRegexes {
    compile_selector_regexes_with_html_ids(attributes, component_attributes, false)
}

pub fn compile_selector_regexes_with_html_ids(
    attributes: &[String],
    component_attributes: &BTreeMap<String, String>,
    html_ids: bool,
) -> SelectorRegexes {
    let mut playwright_attributes: Vec<_> = attributes
        .iter()
        .chain(component_attributes.values())
        .cloned()
        .collect();
    if html_ids {
        playwright_attributes.push(HTML_ID_ATTRIBUTE.to_string());
    }
    playwright_attributes.sort();
    playwright_attributes.dedup();

    SelectorRegexes {
        app_attributes: attributes.to_vec(),
        component_attributes: component_attributes.clone(),
        playwright_attributes: playwright_attributes
            .iter()
            .map(|attribute| AttributeRegex {
                attribute: attribute.clone(),
                regex: playwright_selector_regex(attribute),
            })
            .collect(),
        html_ids,
    }
}

pub(super) fn playwright_selector_regex(attribute: &str) -> Regex {
    let pattern = format!(
        r#"\[\s*{}\s*(=|\^=|\$=|\*=)\s*(?:"([^"]+)"|'([^']+)')\s*\]"#,
        regex::escape(attribute)
    );
    Regex::new(&pattern).expect("valid Playwright selector regex")
}

pub(super) fn matcher_for_operator(operator: &str, value: &str) -> SelectorMatcher {
    match operator {
        "^=" => SelectorMatcher::Prefix(value.to_string()),
        "$=" => SelectorMatcher::Suffix(value.to_string()),
        "*=" => SelectorMatcher::Contains(value.to_string()),
        _ => SelectorMatcher::Exact(value.to_string()),
    }
}

pub(super) fn first_capture<'a>(
    captures: &'a regex::Captures<'_>,
    indexes: &[usize],
) -> Option<&'a str> {
    indexes
        .iter()
        .find_map(|index| captures.get(*index).map(|capture| capture.as_str()))
}
