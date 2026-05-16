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
