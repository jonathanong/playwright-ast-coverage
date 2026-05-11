use crate::js_scan;
#[cfg(test)]
use anyhow::Result;
use regex::Regex;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
#[cfg(test)]
use walkdir::WalkDir;

const SOURCE_EXTS: &[&str] = &["ts", "tsx", "js", "jsx"];

pub struct SelectorRegexes {
    app_attributes: Vec<AttributeRegex>,
    playwright_attributes: Vec<AttributeRegex>,
}

struct AttributeRegex {
    attribute: String,
    regex: Regex,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct AppSelector {
    pub file: PathBuf,
    pub attribute: String,
    pub value: AppSelectorValue,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum AppSelectorValue {
    Exact(String),
    Template(TemplatePattern),
    Unsupported(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TemplatePattern {
    raw: String,
    parts: Vec<String>,
    starts_static: bool,
    ends_static: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PlaywrightSelector {
    pub attribute: String,
    pub selector: String,
    matcher: SelectorMatcher,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum SelectorMatcher {
    Exact(String),
    Prefix(String),
    Suffix(String),
    Contains(String),
    Regex(String),
}

pub fn compile_selector_regexes(attributes: &[String]) -> SelectorRegexes {
    SelectorRegexes {
        app_attributes: attributes
            .iter()
            .map(|attribute| AttributeRegex {
                attribute: attribute.clone(),
                regex: app_selector_regex(attribute),
            })
            .collect(),
        playwright_attributes: attributes
            .iter()
            .map(|attribute| AttributeRegex {
                attribute: attribute.clone(),
                regex: playwright_selector_regex(attribute),
            })
            .collect(),
    }
}

impl AppSelector {
    pub fn display_value(&self) -> String {
        self.value.display_value()
    }

    pub fn unsupported_dynamic(&self) -> bool {
        matches!(self.value, AppSelectorValue::Unsupported(_))
    }

    pub fn matches_playwright(&self, selector: &PlaywrightSelector) -> bool {
        self.attribute == selector.attribute && self.value.matches_selector(&selector.matcher)
    }
}

impl AppSelectorValue {
    fn display_value(&self) -> String {
        match self {
            Self::Exact(value) => value.clone(),
            Self::Template(pattern) => pattern.raw.clone(),
            Self::Unsupported(value) => format!("{{{value}}}"),
        }
    }

    fn matches_selector(&self, matcher: &SelectorMatcher) -> bool {
        match self {
            Self::Exact(value) => matcher.matches_value(value),
            Self::Template(pattern) => matcher.matches_pattern(pattern),
            Self::Unsupported(_) => false,
        }
    }
}

impl TemplatePattern {
    fn new(raw: &str) -> Option<Self> {
        let parts = template_parts(raw);
        if parts.iter().all(|part| part.is_empty()) {
            return None;
        }
        Some(Self {
            raw: raw.to_string(),
            parts,
            starts_static: !raw.starts_with("${"),
            ends_static: !raw.ends_with('}'),
        })
    }

    fn matches_exact(&self, value: &str) -> bool {
        let non_empty: Vec<&str> = self
            .parts
            .iter()
            .filter(|part| !part.is_empty())
            .map(String::as_str)
            .collect();
        if non_empty.is_empty() {
            return false;
        }
        if self.starts_static && !value.starts_with(non_empty[0]) {
            return false;
        }
        if self.ends_static && !value.ends_with(non_empty[non_empty.len() - 1]) {
            return false;
        }

        let mut offset = 0;
        for part in non_empty {
            let Some(index) = value[offset..].find(part) else {
                return false;
            };
            offset += index + part.len();
        }
        true
    }

    fn sample(&self) -> String {
        let mut sample = String::new();
        for (index, part) in self.parts.iter().enumerate() {
            if index > 0 {
                sample.push('x');
            }
            sample.push_str(part);
        }
        sample
    }

    fn first_static(&self) -> Option<&str> {
        self.parts
            .iter()
            .find(|part| !part.is_empty())
            .map(String::as_str)
    }

    fn last_static(&self) -> Option<&str> {
        self.parts
            .iter()
            .rev()
            .find(|part| !part.is_empty())
            .map(String::as_str)
    }

    fn contains_static(&self, needle: &str) -> bool {
        self.parts
            .iter()
            .any(|part| !part.is_empty() && (part.contains(needle) || needle.contains(part)))
    }
}

impl SelectorMatcher {
    fn matches_value(&self, value: &str) -> bool {
        match self {
            Self::Exact(expected) => value == expected,
            Self::Prefix(prefix) => value.starts_with(prefix),
            Self::Suffix(suffix) => value.ends_with(suffix),
            Self::Contains(part) => value.contains(part),
            Self::Regex(pattern) => Regex::new(pattern)
                .map(|regex| regex.is_match(value))
                .unwrap_or(false),
        }
    }

    fn matches_pattern(&self, pattern: &TemplatePattern) -> bool {
        match self {
            Self::Exact(value) => pattern.matches_exact(value),
            Self::Prefix(prefix) => pattern
                .first_static()
                .is_some_and(|part| part.starts_with(prefix) || prefix.starts_with(part)),
            Self::Suffix(suffix) => pattern
                .last_static()
                .is_some_and(|part| part.ends_with(suffix) || suffix.ends_with(part)),
            Self::Contains(part) => pattern.contains_static(part),
            Self::Regex(regex) => Regex::new(regex)
                .map(|regex| regex.is_match(&pattern.sample()))
                .unwrap_or(false),
        }
    }
}

#[cfg(test)]
pub fn collect_app_selectors(
    frontend_root: &Path,
    attributes: &[String],
) -> Result<Vec<AppSelector>> {
    if frontend_root.exists() {
        let mut selectors = BTreeSet::new();
        for entry in WalkDir::new(frontend_root)
            .into_iter()
            .filter_entry(|entry| !is_skipped_dir(entry.path()))
            .filter_map(|entry| entry.ok())
        {
            let path = entry.path();
            if !path.is_file() || !is_source_file(path) {
                continue;
            }
            let source = std::fs::read_to_string(path)?;
            selectors.extend(extract_app_selectors(path, &source, attributes));
        }
        Ok(selectors.into_iter().collect())
    } else {
        Ok(Vec::new())
    }
}

#[cfg(test)]
pub fn extract_app_selectors(path: &Path, source: &str, attributes: &[String]) -> Vec<AppSelector> {
    let regexes = compile_selector_regexes(attributes);
    extract_app_selectors_with_regexes(path, source, &regexes)
}

pub fn extract_app_selectors_with_regexes(
    path: &Path,
    source: &str,
    regexes: &SelectorRegexes,
) -> Vec<AppSelector> {
    let source = js_scan::mask_comments(source);
    let mut selectors = BTreeSet::new();
    for attribute in &regexes.app_attributes {
        for captures in attribute.regex.captures_iter(&source) {
            let value = app_selector_value(&captures);
            selectors.insert(AppSelector {
                file: path.to_path_buf(),
                attribute: attribute.attribute.clone(),
                value,
            });
        }
    }
    selectors.into_iter().collect()
}

#[cfg(test)]
pub fn extract_playwright_selectors(
    source: &str,
    selector_attributes: &[String],
    test_id_attributes: &[String],
) -> Vec<PlaywrightSelector> {
    let regexes = compile_selector_regexes(selector_attributes);
    extract_playwright_selectors_with_regexes(source, &regexes, test_id_attributes)
}

pub fn extract_playwright_selectors_with_regexes(
    source: &str,
    regexes: &SelectorRegexes,
    test_id_attributes: &[String],
) -> Vec<PlaywrightSelector> {
    let source = js_scan::mask_comments(source);
    let mut selectors = BTreeSet::new();
    extract_css_attribute_selectors(&source, &regexes.playwright_attributes, &mut selectors);
    extract_get_by_test_id_selectors(&source, test_id_attributes, &mut selectors);
    selectors.into_iter().collect()
}

fn app_selector_value(captures: &regex::Captures<'_>) -> AppSelectorValue {
    if let Some(value) = first_capture(captures, &[1, 2, 3, 4]) {
        return AppSelectorValue::Exact(value.to_string());
    }
    if let Some(value) = captures.get(5).map(|capture| capture.as_str()) {
        return TemplatePattern::new(value)
            .map(AppSelectorValue::Template)
            .unwrap_or_else(|| AppSelectorValue::Unsupported(value.to_string()));
    }
    captures
        .get(6)
        .map(|capture| AppSelectorValue::Unsupported(capture.as_str().trim().to_string()))
        .unwrap_or_else(|| AppSelectorValue::Unsupported(String::new()))
}

fn extract_css_attribute_selectors(
    source: &str,
    attributes: &[AttributeRegex],
    selectors: &mut BTreeSet<PlaywrightSelector>,
) {
    for attribute in attributes {
        for captures in attribute.regex.captures_iter(source) {
            let op = captures.get(1).expect("operator capture").as_str();
            let value = first_capture(&captures, &[2, 3]).expect("value capture");
            selectors.insert(PlaywrightSelector {
                attribute: attribute.attribute.clone(),
                selector: captures
                    .get(0)
                    .expect("selector capture")
                    .as_str()
                    .to_string(),
                matcher: matcher_for_operator(op, value),
            });
        }
    }
}

fn extract_get_by_test_id_selectors(
    source: &str,
    attributes: &[String],
    selectors: &mut BTreeSet<PlaywrightSelector>,
) {
    for captures in get_by_test_id_string_regex().captures_iter(source) {
        let value = first_capture(&captures, &[1, 2, 3]).expect("value capture");
        for attribute in attributes {
            selectors.insert(PlaywrightSelector {
                attribute: attribute.clone(),
                selector: format!("getByTestId({value})"),
                matcher: SelectorMatcher::Exact(value.to_string()),
            });
        }
    }

    for captures in get_by_test_id_regex_regex().captures_iter(source) {
        let value = captures.get(1).expect("regex capture").as_str();
        for attribute in attributes {
            selectors.insert(PlaywrightSelector {
                attribute: attribute.clone(),
                selector: format!("getByTestId(/{value}/)"),
                matcher: SelectorMatcher::Regex(value.to_string()),
            });
        }
    }
}

fn app_selector_regex(attribute: &str) -> Regex {
    let pattern = format!(
        r#"{}\s*=\s*(?:"([^"]*)"|'([^']*)'|\{{\s*"([^"]*)"\s*\}}|\{{\s*'([^']*)'\s*\}}|\{{\s*`([^`]*)`\s*\}}|\{{([^}}]*)\}})"#,
        regex::escape(attribute)
    );
    Regex::new(&pattern).expect("valid app selector regex")
}

fn playwright_selector_regex(attribute: &str) -> Regex {
    let pattern = format!(
        r#"\[\s*{}\s*(=|\^=|\$=|\*=)\s*(?:"([^"]+)"|'([^']+)')\s*\]"#,
        regex::escape(attribute)
    );
    Regex::new(&pattern).expect("valid Playwright selector regex")
}

fn get_by_test_id_string_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"\.getByTestId\s*\(\s*(?:'([^']+)'|"([^"]+)"|`([^`]+)`)"#)
            .expect("valid getByTestId string regex")
    })
}

fn get_by_test_id_regex_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"\.getByTestId\s*\(\s*/((?:\\.|[^/])*)/[a-z]*"#)
            .expect("valid getByTestId regex regex")
    })
}

fn matcher_for_operator(operator: &str, value: &str) -> SelectorMatcher {
    match operator {
        "^=" => SelectorMatcher::Prefix(value.to_string()),
        "$=" => SelectorMatcher::Suffix(value.to_string()),
        "*=" => SelectorMatcher::Contains(value.to_string()),
        _ => SelectorMatcher::Exact(value.to_string()),
    }
}

fn first_capture<'a>(captures: &'a regex::Captures<'_>, indexes: &[usize]) -> Option<&'a str> {
    indexes
        .iter()
        .find_map(|index| captures.get(*index).map(|capture| capture.as_str()))
}

fn template_parts(source: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find("${") {
        parts.push(rest[..start].to_string());
        let expression = &rest[start + 2..];
        let Some(end) = expression.find('}') else {
            return vec![source.to_string()];
        };
        rest = &expression[end + 1..];
    }
    parts.push(rest.to_string());
    parts
}

pub fn is_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| SOURCE_EXTS.contains(&extension))
}

#[cfg(test)]
fn is_skipped_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, ".git" | "node_modules" | "target" | "dist" | "build"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attrs() -> Vec<String> {
        vec!["data-testid".to_string(), "data-pw".to_string()]
    }

    #[test]
    fn extracts_static_jsx_selectors() {
        let source = r#"
<button data-testid="save" />
<button data-pw={'publish'} />
<button data-testid={'delete'} />
"#;
        let selectors = extract_app_selectors(Path::new("app/page.tsx"), source, &attrs());
        let mut values: Vec<String> = selectors.iter().map(AppSelector::display_value).collect();
        values.sort();
        assert_eq!(values, vec!["delete", "publish", "save"]);
    }

    #[test]
    fn extracts_template_and_unsupported_jsx_selectors() {
        let source = r#"
<article data-testid={`user-${id}`} />
<button data-pw={id} />
"#;
        let selectors = extract_app_selectors(Path::new("app/page.tsx"), source, &attrs());
        assert!(selectors
            .iter()
            .any(|selector| selector.display_value() == "user-${id}"));
        assert!(selectors.iter().any(AppSelector::unsupported_dynamic));
    }

    #[test]
    fn ignores_app_selectors_inside_comments() {
        let source = r#"
// <button data-testid="commented-line" />
/*
<button data-testid="commented-block" />
*/
<button data-testid="real" />
"#;
        let selectors = extract_app_selectors(Path::new("app/page.tsx"), source, &attrs());
        let values: Vec<String> = selectors.iter().map(AppSelector::display_value).collect();
        assert_eq!(values, vec!["real"]);
    }

    #[test]
    fn collect_app_selectors_reads_source_files_and_skips_build_dirs() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("node_modules/pkg")).unwrap();
        std::fs::write(dir.path().join("page.tsx"), r#"<div data-testid="ok" />"#).unwrap();
        std::fs::write(
            dir.path().join("style.css"),
            r#"[data-testid="ignored"] {}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("node_modules/pkg/page.tsx"),
            r#"<div data-testid="ignored" />"#,
        )
        .unwrap();

        let selectors = collect_app_selectors(dir.path(), &attrs()).unwrap();
        assert_eq!(selectors.len(), 1);
        assert_eq!(selectors[0].display_value(), "ok");
        assert!(collect_app_selectors(&dir.path().join("missing"), &attrs())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn extracts_playwright_css_and_test_id_selectors() {
        let source = r#"
await page.getByTestId('save').click();
await page.locator("[data-testid^='user-']").click();
await page.click('[data-pw$="button"]');
await page.locator('[data-pw*="nav"]');
await page.locator('[data-pw="exact"]');
await page.getByTestId(/^account-/);
"#;
        let selectors =
            extract_playwright_selectors(source, &attrs(), &["data-testid".to_string()]);
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == "getByTestId(save)"));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == "[data-testid^='user-']"));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == r#"[data-pw$="button"]"#));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == r#"[data-pw*="nav"]"#));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == r#"[data-pw="exact"]"#));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == "getByTestId(/^account-/)"));
    }

    #[test]
    fn ignores_playwright_selectors_inside_comments() {
        let source = r#"
// await page.getByTestId('commented-line').click();
/*
await page.locator('[data-testid="commented-block"]').click();
*/
await page.getByTestId('real').click();
"#;
        let selectors =
            extract_playwright_selectors(source, &attrs(), &["data-testid".to_string()]);
        assert_eq!(selectors.len(), 1);
        assert_eq!(selectors[0].selector, "getByTestId(real)");
    }

    #[test]
    fn custom_test_id_attribute_maps_get_by_test_id() {
        let selectors = extract_playwright_selectors(
            "await page.getByTestId(\"save\");",
            &["data-test".to_string()],
            &["data-test".to_string()],
        );
        assert_eq!(selectors[0].attribute, "data-test");
    }

    #[test]
    fn exact_and_operator_matchers_cover_static_values() {
        let app = AppSelector {
            file: PathBuf::from("app/page.tsx"),
            attribute: "data-testid".to_string(),
            value: AppSelectorValue::Exact("save-button".to_string()),
        };
        let selectors = extract_playwright_selectors(
            r#"
page.locator('[data-testid="save-button"]');
page.locator('[data-testid^="save"]');
page.locator('[data-testid$="button"]');
page.locator('[data-testid*="ve-bu"]');
page.getByTestId(/^save-/);
"#,
            &["data-testid".to_string()],
            &["data-testid".to_string()],
        );
        assert!(selectors
            .iter()
            .all(|selector| app.matches_playwright(selector)));
    }

    #[test]
    fn template_matchers_cover_structured_dynamic_values() {
        let app = AppSelector {
            file: PathBuf::from("app/page.tsx"),
            attribute: "data-testid".to_string(),
            value: AppSelectorValue::Template(TemplatePattern::new("user-${id}-button").unwrap()),
        };
        let selectors = extract_playwright_selectors(
            r#"
page.locator('[data-testid="user-123-button"]');
page.locator('[data-testid^="user-"]');
page.locator('[data-testid$="-button"]');
page.locator('[data-testid*="user-"]');
page.getByTestId(/^user-/);
"#,
            &["data-testid".to_string()],
            &["data-testid".to_string()],
        );
        assert!(selectors
            .iter()
            .all(|selector| app.matches_playwright(selector)));
    }

    #[test]
    fn mismatched_attributes_and_values_do_not_cover() {
        let app = AppSelector {
            file: PathBuf::from("app/page.tsx"),
            attribute: "data-testid".to_string(),
            value: AppSelectorValue::Exact("save".to_string()),
        };
        let selectors = extract_playwright_selectors(
            r#"
page.locator('[data-pw="save"]');
page.locator('[data-testid="cancel"]');
page.getByTestId(/[invalid/);
"#,
            &attrs(),
            &["data-testid".to_string()],
        );
        assert!(selectors
            .iter()
            .all(|selector| !app.matches_playwright(selector)));
    }

    #[test]
    fn unsupported_dynamic_values_never_match() {
        let app = AppSelector {
            file: PathBuf::from("app/page.tsx"),
            attribute: "data-testid".to_string(),
            value: AppSelectorValue::Unsupported("id".to_string()),
        };
        let selectors = extract_playwright_selectors(
            "page.getByTestId('anything');",
            &["data-testid".to_string()],
            &["data-testid".to_string()],
        );
        assert!(!app.matches_playwright(&selectors[0]));
        assert_eq!(app.display_value(), "{id}");
    }

    #[test]
    fn malformed_template_is_treated_as_literal_pattern() {
        let pattern = TemplatePattern::new("user-${id").unwrap();
        assert!(pattern.matches_exact("user-${id"));
        assert_eq!(pattern.sample(), "user-${id");
    }

    #[test]
    fn template_without_static_parts_is_unsupported() {
        assert!(TemplatePattern::new("${id}").is_none());
    }

    #[test]
    fn template_exact_matching_rejects_non_matching_values() {
        let pattern = TemplatePattern::new("user-${id}-button").unwrap();
        assert!(!pattern.matches_exact("admin-1-button"));
        assert!(!pattern.matches_exact("user-1-link"));
        assert!(!pattern.matches_exact("user-1"));
        assert!(!pattern.matches_exact("user-button"));
    }

    #[test]
    fn app_selector_value_falls_back_without_value_captures() {
        let captures = Regex::new("x").unwrap().captures("x").unwrap();
        assert_eq!(
            app_selector_value(&captures),
            AppSelectorValue::Unsupported(String::new())
        );
    }

    #[test]
    fn empty_internal_template_pattern_does_not_match() {
        let pattern = TemplatePattern {
            raw: String::new(),
            parts: vec![String::new()],
            starts_static: false,
            ends_static: false,
        };
        assert!(!pattern.matches_exact("anything"));
    }
}
