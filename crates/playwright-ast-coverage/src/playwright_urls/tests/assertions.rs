use crate::playwright_tests::TestStatus;
use crate::playwright_urls::api::{extract_playwright_url_occurrences, extract_playwright_urls};
use crate::playwright_urls::callee::callee_has_not;

#[test]
fn page_url_to_match_requires_positive_page_url_expectation() {
    let urls = extract_playwright_urls(
        r#"
        await expect(page.url()).toMatch(/^\/$/);
        await (expect(page.url())).toMatch(/\/account$/);
        await expect(page.url()).toMatch(/\/settings$/);
        await expect.soft(page.url()).toMatch(/\/soft$/);
        await expect(page.url()).not.toMatch(/\/blocked$/);
        await expect(otherpage.url()).toMatch(/\/other$/);
        await expect(page.title()).toMatch(/\/title$/);
        await assert(page.url()).toMatch(/\/assert$/);
        await getExpect()(page.url()).toMatch(/\/factory$/);
        await helpers.expect(page.url()).toMatch(/\/helper$/);
        await expect('/literal').toMatch(/\/literal$/);
        await expect(page.url()).toMatch(`/users/${role === 'admin' ? '/admin' : '/user'}`);
        await page.toMatch(/\/method$/);
        "#,
    );
    assert_eq!(
        urls,
        vec![
            "/",
            "/account",
            "/settings",
            "/soft",
            "/users/${role === 'admin' ? '/admin' : '/user'}"
        ]
    );
}

#[test]
fn ignores_negative_to_have_url_assertions() {
    let urls = extract_playwright_urls(
        "await expect(page).not.toHaveURL('/settings');\nawait expect(page).toHaveURL('/home');",
    );
    assert_eq!(urls, vec!["/home"]);
    assert!(!callee_has_not(&None));
    assert!(callee_has_not(&Some(vec![
        "not".to_string(),
        "toHaveURL".to_string()
    ])));
}

#[test]
fn marks_urls_inside_skipped_and_conditional_tests() {
    let urls = extract_playwright_url_occurrences(
        r#"
        test.skip('skipped', async ({ page }) => {
            await page.goto('/skipped');
        });
        if (process.env.E2E) {
            test('conditional wrapper', async ({ page }) => {
                await page.goto('/conditional-wrapper');
            });
        } else {
            test('conditional alternate', async ({ page }) => {
                await page.goto('/conditional-alternate');
            });
        }
        featureFlag && test('logical wrapper', async ({ page }) => {
            await page.goto('/logical-wrapper');
        });
        featureFlag
            ? test('ternary consequent', async ({ page }) => {
                await page.goto('/ternary-consequent');
            })
            : test('ternary alternate', async ({ page }) => {
                await page.goto('/ternary-alternate');
            });
        test.skipIf(browserName === 'webkit')('skip if', async ({ page }) => {
            await page.goto('/skip-if');
        });
        test.describe.skip(() => {
            test('describe skip callback', async ({ page }) => {
                await page.goto('/describe-skip-callback');
            });
        });
        test.fixme('fixme test', async ({ page }) => {
            await page.goto('/fixme');
        });
        test('annotation', async ({ page, browserName }) => {
            test.skip(browserName === 'webkit', 'conditional');
            test.fixme(browserName === 'firefox', 'also conditional');
            await page.goto('/conditional-annotation');
        });
        test.describe('scope annotation', () => {
            test.skip(({ browserName }) => browserName === 'webkit', 'conditional');
            test('describe scope annotation', async ({ page }) => {
                await page.goto('/describe-scope-annotation');
            });
        });
        test('conditional skip annotation', async ({ page }) => {
            if (process.env.SKIP_E2E) {
                test.skip();
            }
            await page.goto('/conditional-skip-call');
        });
        test.skip(false, 'skip false', async ({ page }) => {
            await page.goto('/skip-false');
        });
        helpers.skipIf(featureFlag)(async () => {
            await page.goto('/unrelated-skip-if');
        });
        test('active', async ({ page }) => {
            await page.goto('/active');
        });
        test.skip(({ browserName }) => browserName === 'webkit', 'conditional');
        test('file scope annotation', async ({ page }) => {
            await page.goto('/scope-annotation');
        });
        "#,
    );

    assert_eq!(
        urls,
        vec![
            ("/active".to_string(), TestStatus::Active),
            (
                "/conditional-alternate".to_string(),
                TestStatus::Conditional
            ),
            (
                "/conditional-annotation".to_string(),
                TestStatus::Conditional
            ),
            (
                "/conditional-skip-call".to_string(),
                TestStatus::Conditional
            ),
            ("/conditional-wrapper".to_string(), TestStatus::Conditional),
            (
                "/describe-scope-annotation".to_string(),
                TestStatus::Conditional
            ),
            ("/describe-skip-callback".to_string(), TestStatus::Skipped),
            ("/fixme".to_string(), TestStatus::Skipped),
            ("/logical-wrapper".to_string(), TestStatus::Conditional),
            ("/scope-annotation".to_string(), TestStatus::Conditional),
            ("/skip-false".to_string(), TestStatus::Active),
            ("/skip-if".to_string(), TestStatus::Conditional),
            ("/skipped".to_string(), TestStatus::Skipped),
            ("/ternary-alternate".to_string(), TestStatus::Conditional),
            ("/ternary-consequent".to_string(), TestStatus::Conditional),
            ("/unrelated-skip-if".to_string(), TestStatus::Active),
        ]
    );
}
