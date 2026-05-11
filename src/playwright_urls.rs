use crate::js_scan;
use std::collections::BTreeSet;

/// Extract local URL string literals navigated to in a Playwright test file.
///
/// Recognizes:
/// - `page.goto('<url>')`
/// - `page.click('a[href="<url>"]')`
#[cfg(test)]
pub fn extract_playwright_urls(source: &str) -> Vec<String> {
    extract_playwright_url_literals_with_helpers(source, &[])
        .into_iter()
        .filter(|url| url.starts_with('/'))
        .collect()
}

pub fn extract_playwright_url_literals_with_helpers(
    source: &str,
    navigation_helpers: &[String],
) -> Vec<String> {
    let mut urls = BTreeSet::new();

    for arguments in helper_call_arguments(source, ".goto") {
        let Some(url) = first_literal_argument(arguments) else {
            continue;
        };
        if is_candidate_url(&url) {
            urls.insert(url);
        }
    }

    for arguments in helper_call_arguments(source, ".click") {
        let Some(selector) = first_literal_argument(arguments) else {
            continue;
        };
        if let Some(url) = extract_href_from_selector(&selector) {
            urls.insert(url);
        }
    }

    for arguments in helper_call_arguments(source, ".toHaveURL") {
        for url in string_literals_outside_comments(arguments) {
            if is_candidate_url(&url) {
                urls.insert(url);
                break;
            }
        }
    }

    for helper in navigation_helpers {
        for arguments in helper_call_arguments(source, helper) {
            for url in string_literals_outside_comments(arguments) {
                if is_candidate_url(&url) {
                    urls.insert(url);
                    break;
                }
            }
        }
    }

    urls.into_iter().collect()
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

fn helper_call_arguments<'a>(source: &'a str, helper: &str) -> Vec<&'a str> {
    if helper.trim().is_empty() {
        return Vec::new();
    }

    let bytes = source.as_bytes();
    let mut arguments = Vec::new();
    let mut offset = 0;

    while let Some(start) = js_scan::find_outside_syntax(source, helper, offset) {
        let after_helper = start + helper.len();
        if !has_callee_boundary(bytes, start, after_helper, helper.starts_with('.')) {
            offset = after_helper;
            continue;
        }

        let mut open = after_helper;
        while open < bytes.len() && bytes[open].is_ascii_whitespace() {
            open += 1;
        }
        if bytes.get(open) != Some(&b'(') {
            offset = after_helper;
            continue;
        }

        if let Some(close) = find_matching_paren(source, open) {
            arguments.push(&source[open + 1..close]);
            offset = close + 1;
        } else {
            offset = open + 1;
        }
    }

    arguments
}

fn first_literal_argument(source: &str) -> Option<String> {
    let source = js_scan::mask_comments(source);
    let bytes = source.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if !matches!(bytes.get(i), Some(b'\'' | b'"' | b'`')) {
        return None;
    }
    let end = find_string_end(&source, i)?;
    Some(source[i + 1..end].to_string())
}

fn string_literals_outside_comments(source: &str) -> Vec<String> {
    string_literals(&js_scan::mask_comments(source))
}

fn has_callee_boundary(bytes: &[u8], start: usize, end: usize, method_call: bool) -> bool {
    let before_ok = method_call || start == 0 || !is_ident(bytes[start - 1]);
    let after_ok = end >= bytes.len() || !is_ident(bytes[end]);
    before_ok && after_ok
}

fn find_matching_paren(source: &str, open: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut quote: Option<u8> = None;
    let mut escaped = false;

    for (i, byte) in bytes.iter().copied().enumerate().skip(open) {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == q {
                quote = None;
            }
            continue;
        }

        match byte {
            b'\'' | b'"' | b'`' => quote = Some(byte),
            b'(' => depth += 1,
            b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }

    None
}

fn string_literals(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut strings = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if matches!(bytes[i], b'\'' | b'"' | b'`') {
            if let Some(end) = find_string_end(source, i) {
                strings.push(source[i + 1..end].to_string());
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }
    strings
}

fn find_string_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let quote = bytes[start];
    let mut escaped = false;
    for (i, byte) in bytes.iter().enumerate().skip(start + 1) {
        if escaped {
            escaped = false;
        } else if *byte == b'\\' {
            escaped = true;
        } else if *byte == quote {
            return Some(i);
        }
    }
    None
}

fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
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
await page.goto('about:blank');
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
        let urls = extract_playwright_urls(src);
        assert!(urls.is_empty());
    }

    #[test]
    fn empty_file_returns_empty() {
        let urls = extract_playwright_urls("");
        assert!(urls.is_empty());
    }

    #[test]
    fn extracts_configured_navigation_helper_urls() {
        let src = r#"
await navigateTo(page, '/settings');
await testHelpers.openPath(page, "/profile");
await notnavigateTo(page, '/ignored-prefix');
await navigateToSomething(page, '/ignored-suffix');
"#;
        let urls = extract_playwright_url_literals_with_helpers(
            src,
            &["navigateTo".to_string(), "testHelpers.openPath".to_string()],
        );
        assert_eq!(urls, vec!["/profile", "/settings"]);
    }

    #[test]
    fn helper_url_extraction_skips_non_url_literals() {
        let src = r#"
await navigateTo(page, "button name", { fallback: '/fallback' });
await navigateTo(page, getPath('/dynamic'));
"#;
        let urls = extract_playwright_url_literals_with_helpers(src, &["navigateTo".to_string()]);
        assert_eq!(urls, vec!["/dynamic", "/fallback"]);
    }

    #[test]
    fn extracts_to_have_url_assertion_paths() {
        let src = r#"
await expect(page).toHaveURL('/settings');
await expect(page).toHaveURL(new RegExp(`/user/${username}/rss-feed-items/viewed`));
"#;
        let urls = extract_playwright_urls(src);
        assert_eq!(
            urls,
            vec!["/settings", "/user/${username}/rss-feed-items/viewed"]
        );
    }

    #[test]
    fn to_have_url_uses_first_url_literal_argument() {
        let src = r#"
await expect(page).toHaveURL('label', '/settings');
"#;
        let urls = extract_playwright_url_literals_with_helpers(src, &[]);
        assert_eq!(urls, vec!["/settings"]);
    }

    #[test]
    fn helper_argument_scanner_handles_empty_helpers_and_unclosed_calls() {
        assert!(helper_call_arguments("navigateTo(page, '/settings'", "").is_empty());
        assert!(helper_call_arguments("navigateTo(page, '/settings'", "navigateTo").is_empty());
    }

    #[test]
    fn helper_argument_scanner_handles_escaped_quotes() {
        let source = r#"navigateTo(page, "a\"b", '/settings')"#;
        let open = source.find('(').unwrap();
        assert_eq!(find_matching_paren(source, open), Some(source.len() - 1));
        assert_eq!(
            helper_call_arguments(source, "navigateTo"),
            vec![r#"page, "a\"b", '/settings'"#]
        );
    }

    #[test]
    fn string_literal_scanner_handles_escapes_and_unterminated_strings() {
        assert_eq!(
            string_literals(r#""a\"b" '/settings'"#),
            vec![r#"a\"b"#, "/settings"]
        );
        assert!(string_literals("\"unterminated").is_empty());
        assert_eq!(find_string_end(r#""a\"b""#, 0), Some(5));
        assert_eq!(find_string_end("\"unterminated", 0), None);
    }
}
