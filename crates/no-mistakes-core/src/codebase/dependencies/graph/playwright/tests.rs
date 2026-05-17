use super::*;
use std::path::PathBuf;

fn fixture_source(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/ast-snippets/playwright-urls")
        .join(name);
    std::fs::read_to_string(path).expect("playwright URL fixture must be readable")
}

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
fn extracts_page_goto_static_template_literal() {
    let src = r#"
await page.goto(`/settings`);
"#;
    let urls = extract_playwright_urls(src);
    assert_eq!(urls, vec!["/settings"]);
}

#[test]
fn extracts_template_page_goto_url() {
    let src = r#"
const id = '42';
await page.goto(`/users/${id}`);
"#;
    let urls = extract_playwright_urls(src);
    assert_eq!(urls, vec!["/users/:param"]);
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
fn extracts_click_single_quoted_href_selector() {
    let src = r#"
await page.click("a[href='/dashboard']");
"#;
    let urls = extract_playwright_urls(src);
    assert_eq!(urls, vec!["/dashboard"]);
}

#[test]
fn extracts_navigate_to_helper_with_page_argument() {
    let src = r#"
await navigateTo(page, '/settings');
"#;
    let urls = extract_playwright_urls(src);
    assert_eq!(urls, vec!["/settings"]);
}

#[test]
fn extracts_navigate_to_template_literal_with_interpolation() {
    let src = r#"
await navigateTo(page, `/${topicType}/${topicId}/reviews`);
"#;
    let urls = extract_playwright_urls(src);
    assert_eq!(urls, vec!["/:param/:param/reviews"]);
}

#[test]
fn extracts_page_goto_template_literal_with_interpolation() {
    let src = r#"
const response = await page.goto(`/admin/${topicType}/${topicId}/edit`);
"#;
    let urls = extract_playwright_urls(src);
    assert_eq!(urls, vec!["/admin/:param/:param/edit"]);
}

#[test]
fn extracts_click_href_template_literal_with_interpolation() {
    let src = r#"
await page.click(`a[href="/users/${userId}/comments"]`);
"#;
    let urls = extract_playwright_urls(src);
    assert_eq!(urls, vec!["/users/:param/comments"]);
}

#[test]
fn extracts_to_have_url_template_literal_with_interpolation() {
    let src = r#"
await expect(page).toHaveURL(`/communities/${slug}/lists/topics`);
"#;
    let urls = extract_playwright_urls(src);
    assert_eq!(urls, vec!["/communities/:param/lists/topics"]);
}

#[test]
fn extracts_to_have_url_new_regexp_template_literal_with_interpolation() {
    let src = r#"
await expect(page).toHaveURL(new RegExp(`/user/${username}/rss-feed-items/viewed`));
"#;
    let urls = extract_playwright_urls(src);
    assert_eq!(urls, vec!["/user/:param/rss-feed-items/viewed"]);
}

#[test]
fn extracts_wait_for_url_template_literal_with_interpolation() {
    let src = r#"
await page.waitForURL(`/communities/${slug}`);
"#;
    let urls = extract_playwright_urls(src);
    assert_eq!(urls, vec!["/communities/:param"]);
}

#[test]
fn ignores_non_page_goto_or_click_receivers() {
    let src = r#"
await router.goto('/router-route');
await button.click('a[href="/button-route"]');
"#;
    let urls = extract_playwright_urls(src);
    assert!(urls.is_empty());
}

#[test]
fn traverses_common_statement_branches() {
    let src = r#"
if (enabled) {
  await page.goto('/if');
}
try {
  await page.goto('/try');
} catch {
  await page.goto('/catch');
} finally {
  await page.goto('/finally');
}
for (const path of ['/for-of']) {
  await page.goto(path);
}
while (enabled) {
  await page.goto('/while');
}
switch (kind) {
  case 'a':
    await page.goto('/switch');
    break;
}
"#;
    let urls = extract_playwright_urls(src);
    assert_eq!(
        urls,
        vec!["/catch", "/finally", "/if", "/switch", "/try", "/while"]
    );
}

#[test]
fn extracts_navigate_to_helper_with_url_first() {
    let src = r#"
await navigateTo('/dashboard');
"#;
    let urls = extract_playwright_urls(src);
    assert_eq!(urls, vec!["/dashboard"]);
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
fn ignores_data_href_selectors() {
    let src = r#"
await page.click('a[data-href="/dashboard"]');
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
fn covers_ignored_argument_and_selector_shapes() {
    let src = r#"
page.goto();
page.goto(route);
page.waitForURL(route);
page.waitForURL("https://example.com");
page.click("a[href=/missing-quote]");
page.click("a[href=\"https://example.com\"]");
expect(page).toHaveURL();
expect(page).toHaveURL(new RegExp(dynamic));
page.toHaveURL("/not-expect");
expect.soft(page).toHaveURL("/soft-not-supported");
navigateTo(page, "relative");
navigateTo("relative", page);
new URL("/ignored", base);
"#;
    let urls = extract_playwright_urls(src);
    assert!(urls.is_empty());
}

#[test]
fn fixture_walks_statement_and_expression_shapes() {
    let source = fixture_source("walk-all.ts");
    let urls = extract_playwright_urls(&source);

    for expected in [
        "/alternate",
        "/arrow",
        "/block",
        "/conditional",
        "/catch",
        "/do-body",
        "/do-test",
        "/else",
        "/expr-string",
        "/expr-template",
        "/for-body",
        "/for-var-body",
        "/for-var-init",
        "/for-in-body",
        "/for-in-right",
        "/for-init-expr",
        "/for-empty",
        "/for-of-body",
        "/for-of-right",
        "/for-test",
        "/for-update",
        "/finally",
        "/if",
        "/if-test",
        "/logical",
        "/navigate-second",
        "/return",
        "/sequence-one",
        "/sequence-two",
        "/switch-body",
        "/switch-case",
        "/switch-discriminant",
        "/try",
        "/var-init",
        "/while-body",
        "/while-test",
    ] {
        assert!(urls.iter().any(|url| url == expected), "missing {expected}");
    }
    assert!(!urls.iter().any(|url| url.contains("ignored")));
}
