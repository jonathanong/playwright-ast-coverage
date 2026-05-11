use regex::Regex;
use std::collections::BTreeSet;

/// Extract local URL string literals navigated to in a Playwright test file.
///
/// Recognizes:
/// - `page.goto('<url>')`
/// - `page.click('a[href="<url>"]')`
#[cfg(test)]
pub fn extract_playwright_urls(source: &str) -> Vec<String> {
    extract_playwright_url_literals(source)
        .into_iter()
        .filter(|url| url.starts_with('/'))
        .collect()
}

pub fn extract_playwright_url_literals(source: &str) -> Vec<String> {
    let mut urls = BTreeSet::new();

    let goto =
        Regex::new(r#"\.goto\s*\(\s*(?:'([^']+)'|"([^"]+)"|`([^`]+)`)"#).expect("valid goto regex");
    for cap in goto.captures_iter(source) {
        let url = captured_string(&cap);
        if is_candidate_url(url) {
            urls.insert(url.to_string());
        }
    }

    let click = Regex::new(r#"\.click\s*\(\s*(?:'([^']+)'|"([^"]+)"|`([^`]+)`)"#)
        .expect("valid click regex");
    for cap in click.captures_iter(source) {
        let selector = captured_string(&cap);
        if let Some(url) = extract_href_from_selector(selector) {
            urls.insert(url);
        }
    }

    urls.into_iter().collect()
}

fn captured_string<'h>(cap: &regex::Captures<'h>) -> &'h str {
    cap.get(1)
        .or_else(|| cap.get(2))
        .or_else(|| cap.get(3))
        .expect("regex has one string capture")
        .as_str()
}

fn is_candidate_url(url: &str) -> bool {
    url.starts_with('/') || url.starts_with("http://") || url.starts_with("https://")
}

/// Parse `a[href="/users/42"]` to `/users/42`.
fn extract_href_from_selector(selector: &str) -> Option<String> {
    let quoted = selector
        .split("href=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next());
    let single_quoted = selector
        .split("href='")
        .nth(1)
        .and_then(|rest| rest.split('\'').next());
    let url = quoted.or(single_quoted)?;
    if is_candidate_url(url) {
        Some(url.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_page_goto_url() {
        let src = r#"
import { test } from '@playwright/test';
test('view user', async ({ page }) => {
    await page.goto('/users/42');
});
"#;
        let urls = extract_playwright_urls(src);
        assert_eq!(urls, vec!["/users/42"]);
    }

    #[test]
    fn extracts_click_href_selector() {
        let src = r#"
await page.click('a[href="/dashboard"]');
"#;
        let urls = extract_playwright_urls(src);
        assert_eq!(urls, vec!["/dashboard"]);
    }

    #[test]
    fn extracts_double_quoted_goto_and_backtick_single_quoted_href() {
        let src = r#"
await page.goto("/double");
await page.click(`a[href='/single']`);
"#;
        let urls = extract_playwright_urls(src);
        assert_eq!(urls, vec!["/double", "/single"]);
    }

    #[test]
    fn deduplicates_urls() {
        let src = r#"
await page.goto('/users/1');
await page.goto('/users/1');
"#;
        let urls = extract_playwright_urls(src);
        assert_eq!(urls, vec!["/users/1"]);
    }

    #[test]
    fn ignores_external_urls() {
        let src = r#"
await page.goto('https://example.com/page');
"#;
        let urls = extract_playwright_urls(src);
        assert!(urls.is_empty());
    }

    #[test]
    fn ignores_non_href_selectors() {
        let src = r#"
await page.click('button.submit');
"#;
        let urls = extract_playwright_urls(src);
        assert!(urls.is_empty());
    }

    #[test]
    fn ignores_non_url_href_selector() {
        let src = r#"
await page.click('a[href="mailto:test@example.com"]');
"#;
        let urls = extract_playwright_url_literals(src);
        assert!(urls.is_empty());
    }

    #[test]
    fn empty_file_returns_empty() {
        let urls = extract_playwright_urls("");
        assert!(urls.is_empty());
    }
}
