use super::callee::{
    callee_has_not, callee_is_member_named, callee_is_page_url_to_match,
    callee_is_playwright_wait_for_url, callee_matches_navigation_helper, is_candidate_url,
};
use super::literals::{
    argument_candidate_literals, argument_literals, direct_url_pattern_literals,
    extract_href_from_selector,
};
use crate::{ast, playwright_tests};
use oxc_ast::ast::{
    CallExpression, ConditionalExpression, IfStatement, LogicalExpression, Program,
};
use oxc_ast_visit::{walk, Visit};
use std::collections::{BTreeSet, HashMap};

pub(super) struct UrlVisitor<'a, 'h> {
    pub source: &'a str,
    pub navigation_helpers: &'h [String],
    pub static_zero_arg_paths: &'h HashMap<String, Vec<String>>,
    pub status: playwright_tests::TestStatus,
    pub annotation_status: playwright_tests::TestStatus,
    pub urls: BTreeSet<playwright_tests::TestOccurrence<String>>,
    pub current_test_name: Option<String>,
    pub describe_stack: Vec<String>,
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
        if let Some((callback_index, callback_status)) = traversal {
            if let Some(describe) = playwright_tests::describe_name(call) {
                self.describe_stack.push(describe);
                for (index, argument) in call.arguments.iter().enumerate() {
                    if index == callback_index {
                        self.with_status(callback_status, |visitor| {
                            visitor
                                .with_annotation_scope(|visitor| visitor.visit_argument(argument));
                        });
                    } else {
                        self.visit_argument(argument);
                    }
                }
                self.describe_stack.pop();
            } else {
                let test_name = playwright_tests::test_callback_identity(call);
                let previous_test_name = self.current_test_name.clone();
                if test_name.is_some() {
                    self.current_test_name = test_name;
                }
                for (index, argument) in call.arguments.iter().enumerate() {
                    if index == callback_index {
                        self.with_status(callback_status, |visitor| {
                            visitor
                                .with_annotation_scope(|visitor| visitor.visit_argument(argument));
                        });
                    } else {
                        self.visit_argument(argument);
                    }
                }
                self.current_test_name = previous_test_name;
            }
        } else {
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
    pub fn insert(&mut self, value: String) {
        self.urls.insert(playwright_tests::TestOccurrence {
            value,
            status: self.status.merge(self.annotation_status),
            test_name: self.current_test_name.clone(),
            describe_path: self.describe_stack.clone(),
        });
    }

    pub fn with_status(
        &mut self,
        status: playwright_tests::TestStatus,
        visit: impl FnOnce(&mut Self),
    ) {
        let previous = self.status;
        self.status = previous.merge(status);
        visit(self);
        self.status = previous;
    }

    pub fn with_annotation_scope(&mut self, visit: impl FnOnce(&mut Self)) {
        let previous = self.annotation_status;
        self.annotation_status = playwright_tests::TestStatus::Active;
        visit(self);
        self.annotation_status = previous;
    }

    pub fn apply_annotation_call(&mut self, call: &CallExpression<'_>) {
        if let Some(status) = playwright_tests::annotation_status_for_call(call) {
            let status = playwright_tests::merge_annotation_status(self.status, status);
            self.annotation_status =
                playwright_tests::merge_annotation_status(self.annotation_status, status);
        }
    }
}

pub fn extract_playwright_url_occurrences_from_program(
    program: &Program<'_>,
    source: &str,
    navigation_helpers: &[String],
) -> Vec<playwright_tests::TestOccurrence<String>> {
    use super::statics::collect_static_zero_arg_paths;
    use oxc_ast_visit::Visit as _;

    let static_zero_arg_paths = collect_static_zero_arg_paths(source);
    let mut visitor = UrlVisitor {
        source,
        navigation_helpers,
        static_zero_arg_paths: &static_zero_arg_paths,
        status: playwright_tests::TestStatus::Active,
        annotation_status: playwright_tests::TestStatus::Active,
        urls: BTreeSet::new(),
        current_test_name: None,
        describe_stack: Vec::new(),
    };
    visitor.visit_program(program);
    visitor.urls.into_iter().collect()
}
