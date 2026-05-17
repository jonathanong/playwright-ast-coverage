use crate::playwright_urls::api::{
    extract_playwright_url_literals_with_helpers, extract_playwright_urls,
};
use crate::test_support::fixture_source;

#[test]
fn callee_checks_handle_non_member_expressions() {
    let src = "goto('/')";
    let urls = extract_playwright_urls(src);
    assert!(urls.is_empty());
}

#[test]
fn extracts_page_goto_url() {
    let src = fixture_source(&["ast-snippets", "playwright-urls", "page-goto.ts"]);
    let urls = extract_playwright_urls(&src);
    assert_eq!(urls, vec!["/users/42"]);
}

#[test]
fn extracts_click_href_selector() {
    let src = fixture_source(&["ast-snippets", "playwright-urls", "click-href.ts"]);
    let urls = extract_playwright_urls(&src);
    assert_eq!(urls, vec!["/dashboard"]);
}

#[test]
fn extracts_double_quoted_goto_and_backtick_single_quoted_href() {
    let src = fixture_source(&["ast-snippets", "playwright-urls", "quoted-goto-click.ts"]);
    let urls = extract_playwright_urls(&src);
    assert_eq!(urls, vec!["/double", "/single"]);
}

#[test]
fn deduplicates_urls() {
    let src = fixture_source(&["ast-snippets", "playwright-urls", "duplicate-goto.ts"]);
    let urls = extract_playwright_urls(&src);
    assert_eq!(urls, vec!["/users/1"]);
}

#[test]
fn ignores_external_urls() {
    let src = fixture_source(&["ast-snippets", "playwright-urls", "external-urls.ts"]);
    let urls = extract_playwright_urls(&src);
    assert!(urls.is_empty());
}

#[test]
fn ignores_non_href_selectors() {
    let src = fixture_source(&["ast-snippets", "playwright-urls", "non-href-click.ts"]);
    let urls = extract_playwright_urls(&src);
    assert!(urls.is_empty());
}

#[test]
fn ignores_non_url_href_selector() {
    let src = fixture_source(&["ast-snippets", "playwright-urls", "non-url-href-click.ts"]);
    let urls = extract_playwright_urls(&src);
    assert!(urls.is_empty());
}

#[test]
fn empty_file_returns_empty() {
    let urls = extract_playwright_urls("");
    assert!(urls.is_empty());
}

#[test]
fn extracts_configured_navigation_helper_urls() {
    let src = fixture_source(&["ast-snippets", "playwright-urls", "navigation-helpers.ts"]);
    let urls = extract_playwright_url_literals_with_helpers(
        &src,
        &["navigateTo".to_string(), "testHelpers.openPath".to_string()],
    );
    assert_eq!(urls, vec!["/profile", "/settings", "/team"]);
}

#[test]
fn helper_url_extraction_skips_non_url_literals() {
    let src = fixture_source(&["ast-snippets", "playwright-urls", "helper-nested-url.ts"]);
    let urls = extract_playwright_url_literals_with_helpers(&src, &["navigateTo".to_string()]);
    assert_eq!(urls, vec!["/dynamic"]);
}

#[test]
fn navigation_helpers_use_only_the_target_argument() {
    let urls = extract_playwright_url_literals_with_helpers(
        "navigateTo('/orders', { redirect: '/login' });",
        &["navigateTo".to_string()],
    );
    assert_eq!(urls, vec!["/orders"]);
}

#[test]
fn extracts_to_have_url_assertion_paths() {
    let src = fixture_source(&["ast-snippets", "playwright-urls", "to-have-url.ts"]);
    let urls = extract_playwright_urls(&src);
    assert_eq!(
        urls,
        vec!["/settings", "/user/${username}/rss-feed-items/viewed"]
    );
}

#[test]
fn extracts_wait_for_url_page_url_match_and_static_route_helpers() {
    let urls = extract_playwright_urls(
        r#"
        const routes = {
            details: () => "/orders/42",
            overview: () => '/orders',
            metrics: () => `/orders/metrics`,
            dynamic: (id) => `/orders/${id}`,
        };
        // ghost: () => "/comment-only"
        const ignoredText = "text: () => '/string-only'";
        const account = { path: () => "/account" };
        const settings = { path: () => "/settings" };
        const analytics = { details() { return "/analytics"; } };
        await page.waitForURL(details());
        await page.waitForURL(routes.details());
        await page.waitForURL(analytics.details());
        await page.waitForURL("**/orders/globbed");
        await expect(page.url()).toMatch(overview());
        await expect.soft(page.url()).toMatch(/\/orders\/soft$/);
        await expect(page.url()).toMatch(metrics());
        await expect(page.url()).toMatch(dynamic("42"));
        await page.waitForURL(account.path());
        await page.waitForURL(settings.path());
        await frame.waitForURL(/^https:\/\/example.com\/orders\/absolute$/);
        await page.waitForURL(path());
        await page.waitForURL(getPath()());
        await page.waitForURL(/not-a-path/);
        await app.waitForURL("/unrelated");
        await page.goto();
        await page.goto(routeName);
        await page.waitForURL(ghost());
        await page.waitForURL(text());
        "#,
    );
    assert_eq!(
        urls,
        vec![
            "/orders",
            "/orders/42",
            "/orders/globbed",
            "/orders/metrics",
            "/orders/soft",
        ]
    );
}

#[test]
fn to_have_url_uses_first_url_literal_argument() {
    let src = fixture_source(&["ast-snippets", "playwright-urls", "to-have-url-label.ts"]);
    let urls = extract_playwright_url_literals_with_helpers(&src, &[]);
    assert_eq!(urls, vec!["/settings"]);
}

#[test]
fn parenthesized_callee_is_supported() {
    let src = fixture_source(&["ast-snippets", "playwright-urls", "parenthesized-callee.ts"]);
    let urls = extract_playwright_urls(&src);
    assert_eq!(urls, vec!["/settings"]);
}

#[test]
fn bare_builtin_callees_are_ignored() {
    let src = fixture_source(&["ast-snippets", "playwright-urls", "bare-callees.ts"]);
    let urls = extract_playwright_urls(&src);
    assert!(urls.is_empty());
}
