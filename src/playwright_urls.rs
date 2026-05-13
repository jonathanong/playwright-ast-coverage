use crate::{ast, playwright_tests};
#[cfg(test)]
use anyhow::Result;
use oxc_ast::ast::{
    Argument, CallExpression, ConditionalExpression, IfStatement, LogicalExpression, Program,
};
use oxc_ast_visit::{walk, Visit};
use std::collections::BTreeSet;
#[cfg(test)]
use std::path::Path;

/// Extract local URL string literals navigated to in a Playwright test file.
#[cfg(test)]
pub fn extract_playwright_urls(source: &str) -> Vec<String> {
    extract_playwright_url_literals_with_helpers(source, &[])
        .into_iter()
        .filter(|url| url.starts_with('/'))
        .collect()
}

#[cfg(test)]
pub fn extract_playwright_url_literals_with_helpers(
    source: &str,
    navigation_helpers: &[String],
) -> Vec<String> {
    extract_playwright_url_literals_from_path(Path::new("fixture.ts"), source, navigation_helpers)
        .expect("fixture should parse")
}

#[cfg(test)]
pub fn extract_playwright_url_literals_from_path(
    path: &Path,
    source: &str,
    navigation_helpers: &[String],
) -> Result<Vec<String>> {
    ast::with_program(path, source, |program, source| {
        extract_playwright_url_literals_from_program(program, source, navigation_helpers)
    })
}

#[cfg(test)]
pub fn extract_playwright_url_occurrences(
    source: &str,
) -> Vec<(String, playwright_tests::TestStatus)> {
    ast::with_program(Path::new("fixture.ts"), source, |program, source| {
        extract_playwright_url_occurrences_from_program(program, source, &[])
            .into_iter()
            .map(|occurrence| (occurrence.value, occurrence.status))
            .collect()
    })
    .expect("fixture should parse")
}

#[cfg(test)]
pub fn extract_playwright_url_literals_from_program(
    program: &Program<'_>,
    source: &str,
    navigation_helpers: &[String],
) -> Vec<String> {
    extract_playwright_url_occurrences_from_program(program, source, navigation_helpers)
        .into_iter()
        .map(|occurrence| occurrence.value)
        .collect()
}

pub fn extract_playwright_url_occurrences_from_program(
    program: &Program<'_>,
    source: &str,
    navigation_helpers: &[String],
) -> Vec<playwright_tests::TestOccurrence<String>> {
    let mut visitor = UrlVisitor {
        source,
        navigation_helpers,
        status: playwright_tests::TestStatus::Active,
        annotation_status: playwright_tests::TestStatus::Active,
        urls: BTreeSet::new(),
    };
    visitor.visit_program(program);
    visitor.urls.into_iter().collect()
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

struct UrlVisitor<'a, 'h> {
    source: &'a str,
    navigation_helpers: &'h [String],
    status: playwright_tests::TestStatus,
    annotation_status: playwright_tests::TestStatus,
    urls: BTreeSet<playwright_tests::TestOccurrence<String>>,
}

impl<'a> Visit<'a> for UrlVisitor<'a, '_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        let callee = ast::expression_path(&call.callee);

        if callee_is_member_named(&call.callee, "goto") {
            if let Some(url) = call
                .arguments
                .first()
                .and_then(|arg| argument_literal(arg, self.source))
            {
                if is_candidate_url(&url) {
                    self.insert(url);
                }
            }
        } else if callee_is_member_named(&call.callee, "click") {
            if let Some(selector) = call
                .arguments
                .first()
                .and_then(|arg| argument_literal(arg, self.source))
            {
                if let Some(url) = extract_href_from_selector(&selector) {
                    self.insert(url);
                }
            }
        } else if (callee_is_member_named(&call.callee, "toHaveURL") && !callee_has_not(&callee))
            || callee_matches_navigation_helper(&callee, self.navigation_helpers)
        {
            if let Some(url) = first_candidate_literal(&call.arguments, self.source) {
                self.insert(url);
            }
        }

        let traversal = playwright_tests::test_callback_traversal(call, self.annotation_status);
        if traversal.is_none() {
            let callback_index = playwright_tests::callback_argument_index(call);
            if playwright_tests::annotation_status_for_call(call).is_some() {
                self.apply_annotation_call(call);
                for (index, argument) in call.arguments.iter().enumerate() {
                    if Some(index) != callback_index {
                        self.visit_argument(argument);
                    }
                }
                return;
            }
            walk::walk_call_expression(self, call);
            return;
        }

        let (callback_index, callback_status) = traversal.expect("checked traversal");
        for (index, argument) in call.arguments.iter().enumerate() {
            if index == callback_index {
                self.with_status(callback_status, |visitor| {
                    visitor.with_annotation_scope(|visitor| visitor.visit_argument(argument));
                });
            } else {
                self.visit_argument(argument);
            }
        }
    }

    fn visit_if_statement(&mut self, statement: &IfStatement<'a>) {
        self.visit_expression(&statement.test);
        let status = playwright_tests::status_for_if_branch(self.status);
        self.with_status(status, |visitor| {
            visitor.visit_statement(&statement.consequent);
            if let Some(alternate) = &statement.alternate {
                visitor.visit_statement(alternate);
            }
        });
    }

    fn visit_conditional_expression(&mut self, expression: &ConditionalExpression<'a>) {
        self.visit_expression(&expression.test);
        let status = playwright_tests::status_for_if_branch(self.status);
        self.with_status(status, |visitor| {
            visitor.visit_expression(&expression.consequent);
            visitor.visit_expression(&expression.alternate);
        });
    }

    fn visit_logical_expression(&mut self, expression: &LogicalExpression<'a>) {
        self.visit_expression(&expression.left);
        let status = playwright_tests::status_for_if_branch(self.status);
        self.with_status(status, |visitor| {
            visitor.visit_expression(&expression.right)
        });
    }
}

impl UrlVisitor<'_, '_> {
    fn insert(&mut self, value: String) {
        self.urls.insert(playwright_tests::TestOccurrence {
            value,
            status: self.status.merge(self.annotation_status),
        });
    }

    fn with_status(&mut self, status: playwright_tests::TestStatus, visit: impl FnOnce(&mut Self)) {
        let previous = self.status;
        self.status = previous.merge(status);
        visit(self);
        self.status = previous;
    }

    fn with_annotation_scope(&mut self, visit: impl FnOnce(&mut Self)) {
        let previous = self.annotation_status;
        self.annotation_status = playwright_tests::TestStatus::Active;
        visit(self);
        self.annotation_status = previous;
    }

    fn apply_annotation_call(&mut self, call: &CallExpression<'_>) {
        if let Some(status) = playwright_tests::annotation_status_for_call(call) {
            let status = playwright_tests::merge_annotation_status(self.status, status);
            self.annotation_status =
                playwright_tests::merge_annotation_status(self.annotation_status, status);
        }
    }
}

fn callee_matches_navigation_helper(callee: &Option<Vec<String>>, helpers: &[String]) -> bool {
    let Some(parts) = callee else {
        return false;
    };
    let full_name = parts.join(".");
    helpers.iter().any(|helper| {
        helper == &full_name
            || (!helper.contains('.') && parts.last().is_some_and(|part| part == helper))
    })
}

fn callee_has_not(callee: &Option<Vec<String>>) -> bool {
    let Some(parts) = callee else {
        return false;
    };
    parts.iter().any(|part| part == "not")
}

fn callee_is_member_named(callee: &oxc_ast::ast::Expression<'_>, method: &str) -> bool {
    match callee {
        oxc_ast::ast::Expression::StaticMemberExpression(member) => member.property.name == method,
        _ => false,
    }
}

fn first_candidate_literal(arguments: &[Argument<'_>], source: &str) -> Option<String> {
    let mut visitor = LiteralVisitor {
        source,
        literals: Vec::new(),
    };
    for argument in arguments {
        visitor.visit_argument(argument);
    }
    visitor
        .literals
        .into_iter()
        .find(|url| is_candidate_url(url))
}

struct LiteralVisitor<'a> {
    source: &'a str,
    literals: Vec<String>,
}

impl<'a> Visit<'a> for LiteralVisitor<'a> {
    fn visit_string_literal(&mut self, literal: &oxc_ast::ast::StringLiteral<'a>) {
        self.literals.push(literal.value.to_string());
    }

    fn visit_template_literal(&mut self, template: &oxc_ast::ast::TemplateLiteral<'a>) {
        self.literals
            .push(ast::template_literal_text(template, self.source));
        walk::walk_template_literal(self, template);
    }
}

fn argument_literal(argument: &Argument<'_>, source: &str) -> Option<String> {
    match argument {
        Argument::StringLiteral(literal) => Some(literal.value.to_string()),
        Argument::TemplateLiteral(template) => Some(ast::template_literal_text(template, source)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playwright_tests::TestStatus;
    use crate::test_support::fixture_source;

    #[test]
    fn extracts_page_goto_url() {
        let src = fixture_source(&["playwright_urls", "page-goto.ts"]);
        let urls = extract_playwright_urls(&src);
        assert_eq!(urls, vec!["/users/42"]);
    }

    #[test]
    fn extracts_click_href_selector() {
        let src = fixture_source(&["playwright_urls", "click-href.ts"]);
        let urls = extract_playwright_urls(&src);
        assert_eq!(urls, vec!["/dashboard"]);
    }

    #[test]
    fn extracts_double_quoted_goto_and_backtick_single_quoted_href() {
        let src = fixture_source(&["playwright_urls", "quoted-goto-click.ts"]);
        let urls = extract_playwright_urls(&src);
        assert_eq!(urls, vec!["/double", "/single"]);
    }

    #[test]
    fn deduplicates_urls() {
        let src = fixture_source(&["playwright_urls", "duplicate-goto.ts"]);
        let urls = extract_playwright_urls(&src);
        assert_eq!(urls, vec!["/users/1"]);
    }

    #[test]
    fn ignores_external_urls() {
        let src = fixture_source(&["playwright_urls", "external-urls.ts"]);
        let urls = extract_playwright_urls(&src);
        assert!(urls.is_empty());
    }

    #[test]
    fn ignores_non_href_selectors() {
        let src = fixture_source(&["playwright_urls", "non-href-click.ts"]);
        let urls = extract_playwright_urls(&src);
        assert!(urls.is_empty());
    }

    #[test]
    fn ignores_non_url_href_selector() {
        let src = fixture_source(&["playwright_urls", "non-url-href-click.ts"]);
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
        let src = fixture_source(&["playwright_urls", "navigation-helpers.ts"]);
        let urls = extract_playwright_url_literals_with_helpers(
            &src,
            &["navigateTo".to_string(), "testHelpers.openPath".to_string()],
        );
        assert_eq!(urls, vec!["/profile", "/settings", "/team"]);
    }

    #[test]
    fn helper_url_extraction_skips_non_url_literals() {
        let src = fixture_source(&["playwright_urls", "helper-nested-url.ts"]);
        let urls = extract_playwright_url_literals_with_helpers(&src, &["navigateTo".to_string()]);
        assert_eq!(urls, vec!["/dynamic", "/fallback"]);
    }

    #[test]
    fn extracts_to_have_url_assertion_paths() {
        let src = fixture_source(&["playwright_urls", "to-have-url.ts"]);
        let urls = extract_playwright_urls(&src);
        assert_eq!(
            urls,
            vec!["/settings", "/user/${username}/rss-feed-items/viewed"]
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
    fn to_have_url_uses_first_url_literal_argument() {
        let src = fixture_source(&["playwright_urls", "to-have-url-label.ts"]);
        let urls = extract_playwright_url_literals_with_helpers(&src, &[]);
        assert_eq!(urls, vec!["/settings"]);
    }

    #[test]
    fn parenthesized_callee_is_supported() {
        let src = fixture_source(&["playwright_urls", "parenthesized-callee.ts"]);
        let urls = extract_playwright_urls(&src);
        assert_eq!(urls, vec!["/settings"]);
    }

    #[test]
    fn bare_builtin_callees_are_ignored() {
        let src = fixture_source(&["playwright_urls", "bare-callees.ts"]);
        let urls = extract_playwright_urls(&src);
        assert!(urls.is_empty());
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
}
