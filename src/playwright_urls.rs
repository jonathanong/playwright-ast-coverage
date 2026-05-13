use crate::{ast, playwright_tests};
#[cfg(test)]
use anyhow::Result;
use oxc_ast::ast::{
    Argument, CallExpression, ConditionalExpression, IfStatement, LogicalExpression, Program,
};
use oxc_ast_visit::{walk, Visit};
use std::collections::{BTreeSet, HashMap};
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
    let static_zero_arg_paths = collect_static_zero_arg_paths(source);
    let mut visitor = UrlVisitor {
        source,
        navigation_helpers,
        static_zero_arg_paths: &static_zero_arg_paths,
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
    static_zero_arg_paths: &'h HashMap<String, Vec<String>>,
    status: playwright_tests::TestStatus,
    annotation_status: playwright_tests::TestStatus,
    urls: BTreeSet<playwright_tests::TestOccurrence<String>>,
}

impl<'a> Visit<'a> for UrlVisitor<'a, '_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        let callee = ast::expression_path(&call.callee);

        if callee_is_member_named(&call.callee, "goto") {
            if let Some(argument) = call.arguments.first() {
                for url in argument_literals(argument, self.source, self.static_zero_arg_paths) {
                    if is_candidate_url(&url) {
                        self.insert(url);
                    }
                }
            }
        } else if callee_is_member_named(&call.callee, "click") {
            if let Some(selector) = call.arguments.first().and_then(|arg| {
                argument_literals(arg, self.source, self.static_zero_arg_paths)
                    .into_iter()
                    .next()
            }) {
                if let Some(url) = extract_href_from_selector(&selector) {
                    self.insert(url);
                }
            }
        } else if (callee_is_member_named(&call.callee, "toHaveURL") && !callee_has_not(&callee))
            || (callee_is_playwright_wait_for_url(&call.callee) && !callee_has_not(&callee))
            || (callee_is_page_url_to_match(&call.callee) && !callee_has_not(&callee))
        {
            for url in direct_url_pattern_literals(
                &call.arguments,
                self.source,
                self.static_zero_arg_paths,
            ) {
                self.insert(url);
            }
        } else if callee_matches_navigation_helper(&callee, self.navigation_helpers) {
            for argument in &call.arguments {
                let urls =
                    argument_candidate_literals(argument, self.source, self.static_zero_arg_paths);
                if !urls.is_empty() {
                    for url in urls {
                        self.insert(url);
                    }
                    break;
                }
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

fn callee_is_playwright_wait_for_url(callee: &oxc_ast::ast::Expression<'_>) -> bool {
    let oxc_ast::ast::Expression::StaticMemberExpression(member) = callee else {
        return false;
    };
    if member.property.name != "waitForURL" {
        return false;
    }

    ast::expression_path(&member.object).is_some_and(|path| {
        path.last()
            .is_some_and(|receiver| matches!(receiver.as_str(), "page" | "frame"))
    })
}

fn callee_is_page_url_to_match(callee: &oxc_ast::ast::Expression<'_>) -> bool {
    let oxc_ast::ast::Expression::StaticMemberExpression(member) = callee else {
        return false;
    };
    if member.property.name != "toMatch" {
        return false;
    }

    let Some(expect_call) = expect_call_expression(&member.object) else {
        return false;
    };
    let Some(expect_callee) = ast::expression_path(&expect_call.callee) else {
        return false;
    };
    if !matches!(
        expect_callee
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice(),
        ["expect"] | ["expect", "soft"]
    ) {
        return false;
    }

    let Some(Argument::CallExpression(url_call)) = expect_call.arguments.first() else {
        return false;
    };
    ast::expression_path(&url_call.callee).is_some_and(|path| path == ["page", "url"])
}

fn expect_call_expression<'a>(
    expression: &'a oxc_ast::ast::Expression<'a>,
) -> Option<&'a CallExpression<'a>> {
    match expression {
        oxc_ast::ast::Expression::CallExpression(call) => Some(call),
        oxc_ast::ast::Expression::StaticMemberExpression(member)
            if member.property.name == "not" =>
        {
            expect_call_expression(&member.object)
        }
        oxc_ast::ast::Expression::ParenthesizedExpression(parenthesized) => {
            expect_call_expression(&parenthesized.expression)
        }
        _ => None,
    }
}

fn candidate_literals(
    arguments: &[Argument<'_>],
    source: &str,
    static_zero_arg_paths: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    let mut visitor = LiteralVisitor {
        source,
        static_zero_arg_paths,
        literals: Vec::new(),
    };
    for argument in arguments {
        visitor.visit_argument(argument);
    }
    visitor
        .literals
        .into_iter()
        .filter(|url| is_candidate_url(url))
        .collect()
}

fn direct_url_pattern_literals(
    arguments: &[Argument<'_>],
    source: &str,
    static_zero_arg_paths: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    let mut visitor = LiteralVisitor {
        source,
        static_zero_arg_paths,
        literals: Vec::new(),
    };
    for argument in arguments {
        visitor.visit_argument(argument);
    }
    visitor
        .literals
        .into_iter()
        .filter_map(|url| normalize_url_pattern(&url))
        .collect()
}

fn normalize_url_pattern(url: &str) -> Option<String> {
    if is_candidate_url(url) && !url.contains('*') {
        Some(url.to_string())
    } else {
        glob_url_sample(url)
    }
}

fn glob_url_sample(glob: &str) -> Option<String> {
    if !glob.contains('*') {
        return None;
    }

    let (without_scheme, was_leading_wildcard) = glob
        .strip_prefix("**/")
        .map(|value| (value, true))
        .or_else(|| glob.strip_prefix("*/").map(|value| (value, true)))
        .unwrap_or((glob, false));
    let candidate = if is_candidate_url(glob) {
        glob.to_string()
    } else if was_leading_wildcard {
        format!("/{}", without_scheme.trim_start_matches('/'))
    } else if let Some(first_slash) = without_scheme.find('/') {
        let first_segment = &without_scheme[..first_slash];
        if first_segment.contains('.') {
            format!(
                "/{}",
                without_scheme[first_slash + 1..].trim_start_matches('/')
            )
        } else {
            format!("/{}", without_scheme.trim_start_matches('/'))
        }
    } else {
        format!("/{without_scheme}")
    };
    if candidate == "/" || candidate.contains("${") {
        return None;
    }

    let mut sample = String::new();
    let mut chars = candidate.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '*' {
            while matches!(chars.peek(), Some('*')) {
                chars.next();
            }
            sample.push('x');
        } else {
            sample.push(ch);
        }
    }

    is_candidate_url(&sample).then_some(sample)
}

struct LiteralVisitor<'a> {
    source: &'a str,
    static_zero_arg_paths: &'a HashMap<String, Vec<String>>,
    literals: Vec<String>,
}

impl<'a> Visit<'a> for LiteralVisitor<'a> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        let urls = static_zero_arg_path_call(call, self.static_zero_arg_paths);
        if !urls.is_empty() {
            self.literals.extend(urls);
            return;
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_string_literal(&mut self, literal: &oxc_ast::ast::StringLiteral<'a>) {
        self.literals.push(literal.value.to_string());
    }

    fn visit_reg_exp_literal(&mut self, literal: &oxc_ast::ast::RegExpLiteral<'a>) {
        if let Some(sample) = regex_path_sample(literal.regex.pattern.text.as_str()) {
            self.literals.push(sample);
        }
    }

    fn visit_template_literal(&mut self, template: &oxc_ast::ast::TemplateLiteral<'a>) {
        self.literals
            .push(ast::template_literal_text(template, self.source));
    }
}

fn argument_literals(
    argument: &Argument<'_>,
    source: &str,
    static_zero_arg_paths: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    match argument {
        Argument::StringLiteral(literal) => vec![literal.value.to_string()],
        Argument::TemplateLiteral(template) => vec![ast::template_literal_text(template, source)],
        Argument::CallExpression(call) => static_zero_arg_path_call(call, static_zero_arg_paths),
        _ => Vec::new(),
    }
}

fn argument_candidate_literals(
    argument: &Argument<'_>,
    source: &str,
    static_zero_arg_paths: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    match argument {
        Argument::ObjectExpression(_) => Vec::new(),
        _ => candidate_literals(
            std::slice::from_ref(argument),
            source,
            static_zero_arg_paths,
        ),
    }
}

fn collect_static_zero_arg_paths(source: &str) -> HashMap<String, Vec<String>> {
    let pattern = regex::Regex::new(
        r#"([A-Za-z_$][\w$]*)\s*:\s*\(\s*\)\s*=>\s*(?:"([^"`]+)"|'([^'`]+)'|`([^'"`]+)`)"#,
    )
    .expect("static route helper regex should compile");
    let mut candidates: HashMap<String, (Vec<String>, usize)> = HashMap::new();
    for captures in pattern.captures_iter(source) {
        let full_match = captures.get(0).expect("full capture should exist");
        if !source_offset_is_code(source, full_match.start()) {
            continue;
        }
        if let Some((name, value)) = (|| {
            let name = captures.get(1)?;
            let value = captures
                .get(2)
                .or_else(|| captures.get(3))
                .or_else(|| captures.get(4))?;
            Some((name.as_str().to_string(), value.as_str().to_string()))
        })() {
            let entry = candidates.entry(name).or_default();
            entry.0.push(value);
            entry.1 += 1;
        }
    }
    candidates
        .into_iter()
        .filter_map(|(name, (values, count))| (count == 1).then_some((name, values)))
        .collect()
}

fn static_zero_arg_path_call(
    call: &CallExpression<'_>,
    static_zero_arg_paths: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    if !call.arguments.is_empty() {
        return Vec::new();
    }
    let Some(path) = ast::expression_path(&call.callee) else {
        return Vec::new();
    };
    if path.len() != 1 {
        return Vec::new();
    }
    let name = &path[path.len() - 1];
    static_zero_arg_paths
        .get(name.as_str())
        .cloned()
        .unwrap_or_default()
}

fn source_offset_is_code(source: &str, offset: usize) -> bool {
    let mut chars = source.char_indices().peekable();
    let mut in_single = false;
    let mut in_double = false;
    let mut in_template = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut escaped = false;

    while let Some((index, ch)) = chars.next() {
        if index >= offset {
            return !in_single
                && !in_double
                && !in_template
                && !in_line_comment
                && !in_block_comment;
        }

        if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
            }
            continue;
        }
        if in_block_comment {
            if ch == '*' && chars.peek().is_some_and(|(_, next)| *next == '/') {
                chars.next();
                in_block_comment = false;
            }
            continue;
        }
        if escaped {
            escaped = false;
            continue;
        }
        if (in_single || in_double || in_template) && ch == '\\' {
            escaped = true;
            continue;
        }
        if in_single {
            in_single = ch != '\'';
            continue;
        }
        if in_double {
            in_double = ch != '"';
            continue;
        }
        if in_template {
            in_template = ch != '`';
            continue;
        }

        if ch == '/' && chars.peek().is_some_and(|(_, next)| *next == '/') {
            chars.next();
            in_line_comment = true;
        } else if ch == '/' && chars.peek().is_some_and(|(_, next)| *next == '*') {
            chars.next();
            in_block_comment = true;
        } else if ch == '\'' {
            in_single = true;
        } else if ch == '"' {
            in_double = true;
        } else if ch == '`' {
            in_template = true;
        }
    }

    !in_single && !in_double && !in_template && !in_line_comment && !in_block_comment
}

fn regex_path_sample(pattern: &str) -> Option<String> {
    let pattern = pattern.trim_start_matches('^').replace(r"\/", "/");
    let mut chars = pattern.chars().peekable();
    let mut sample = String::new();
    let mut started = pattern.starts_with("http://") || pattern.starts_with("https://");
    let mut unsupported = false;
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let Some(next) = chars.next() else {
                break;
            };
            if started && is_literal_path_char(next) && !next.is_ascii_alphanumeric() {
                sample.push(next);
            } else if started {
                unsupported = true;
                break;
            }
            continue;
        }

        if !started {
            if ch == '/' {
                started = true;
                sample.push(ch);
            }
            continue;
        }

        match ch {
            '[' => {
                consume_regex_char_class(&mut chars);
                sample.push('x');
                consume_regex_quantifier(&mut chars);
            }
            '.' => {
                if sample_is_absolute_url_host(&sample) {
                    sample.push('.');
                } else {
                    sample.push('x');
                }
                consume_regex_quantifier(&mut chars);
            }
            '$' => break,
            '|' | '(' | ')' => {
                unsupported = true;
                break;
            }
            ch if is_literal_path_char(ch) => sample.push(ch),
            _ => {
                unsupported = true;
                break;
            }
        }
    }

    if !unsupported && is_candidate_url(&sample) {
        Some(sample)
    } else {
        None
    }
}

fn consume_regex_char_class(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    let mut escaped = false;
    for next in chars.by_ref() {
        if escaped {
            escaped = false;
            continue;
        }
        if next == '\\' {
            escaped = true;
            continue;
        }
        if next == ']' {
            break;
        }
    }
}

fn consume_regex_quantifier(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    while matches!(chars.peek(), Some('+' | '*' | '?' | '{')) {
        let quantifier = chars.next();
        if quantifier == Some('{') {
            for next in chars.by_ref() {
                if next == '}' {
                    break;
                }
            }
        }
    }
}

fn is_literal_path_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '/' | '-' | '_' | '.' | '~' | '%' | ':')
}

fn sample_is_absolute_url_host(sample: &str) -> bool {
    let Some(after_scheme) = sample
        .strip_prefix("http://")
        .or_else(|| sample.strip_prefix("https://"))
    else {
        return false;
    };
    !after_scheme.contains('/')
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
        let src = fixture_source(&["playwright_urls", "to-have-url.ts"]);
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
    fn samples_simple_url_regex_literals() {
        assert_eq!(
            regex_path_sample(r#"^/orders/[a-z\]]+/.{2,4}$"#),
            Some("/orders/x/x".to_string())
        );
        assert_eq!(
            regex_path_sample(r#"^\/orders\/.*?$"#),
            Some("/orders/x".to_string())
        );
        assert_eq!(
            regex_path_sample(r#"^\/orders\/\%bad$"#),
            Some("/orders/%bad".to_string())
        );
        assert_eq!(regex_path_sample(r#"^\/orders\/\#$"#), None);
        assert_eq!(
            regex_path_sample(r#"^\s/orders$"#),
            Some("/orders".to_string())
        );
        assert_eq!(
            regex_path_sample(r#"^\/orders\/\"#),
            Some("/orders/".to_string())
        );
        assert_eq!(
            regex_path_sample(r#"^https:\/\/example.com\/orders$"#),
            Some("https://example.com/orders".to_string())
        );
        assert_eq!(
            regex_path_sample(r#"^https:\/\/example\.com\/orders$"#),
            Some("https://example.com/orders".to_string())
        );
        assert_eq!(regex_path_sample(r#"^/orders/<id>$"#), None);
        assert_eq!(regex_path_sample(r#"^/orders/(\d+)$"#), None);
        assert_eq!(regex_path_sample(r#"^\/users\/\d+$"#), None);
        assert_eq!(regex_path_sample(r#"^not-a-path$"#), None);
        assert_eq!(regex_path_sample(r#"^/$"#), Some("/".to_string()));
        assert_eq!(regex_path_sample(r#"^\/$"#), Some("/".to_string()));
    }

    #[test]
    fn samples_glob_url_patterns() {
        assert_eq!(
            glob_url_sample("**/orders/*/details"),
            Some("/orders/x/details".to_string())
        );
        assert_eq!(
            glob_url_sample("*/orders/**"),
            Some("/orders/x".to_string())
        );
        assert_eq!(
            glob_url_sample("https://example.com/orders/**"),
            Some("https://example.com/orders/x".to_string())
        );
        assert_eq!(
            glob_url_sample("example.com/orders/**"),
            Some("/orders/x".to_string())
        );
        assert_eq!(glob_url_sample("orders/**"), Some("/orders/x".to_string()));
        assert_eq!(glob_url_sample("orders*"), Some("/ordersx".to_string()));
        assert_eq!(glob_url_sample("**/"), None);
        assert_eq!(glob_url_sample("**/${path}"), None);
        assert_eq!(glob_url_sample("/orders"), None);
    }

    #[test]
    fn source_offset_filter_ignores_comments_and_strings() {
        let source = "'route: () => \\'/string\\'';\n/* route: () => '/block' */\n// route: () => '/line'\nconst route = () => '/real';";
        assert!(!source_offset_is_code(
            source,
            source.find("route:").unwrap()
        ));
        assert!(!source_offset_is_code(
            source,
            source.find("block").unwrap() - 14
        ));
        assert!(!source_offset_is_code(
            source,
            source.find("line").unwrap() - 14
        ));
        assert!(source_offset_is_code(
            source,
            source.rfind("route").unwrap()
        ));
        assert!(!source_offset_is_code(
            "'unterminated",
            "'unterminated".len()
        ));
    }

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
