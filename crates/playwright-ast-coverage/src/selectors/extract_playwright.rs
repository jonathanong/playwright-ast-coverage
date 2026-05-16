use super::call_shapes::{
    callee_is_static_member_named, extract_get_by_test_id_call, selector_argument_literals,
    selector_argument_mode,
};
use super::css::{extract_css_attribute_selectors, extract_css_id_selectors};
use super::types::{PlaywrightSelector, SelectorRegexes};
use crate::playwright_tests;
use oxc_ast_visit::Visit;

pub fn extract_playwright_selector_occurrences_from_program(
    program: &oxc_ast::ast::Program<'_>,
    source: &str,
    regexes: &SelectorRegexes,
    test_id_attributes: &[String],
) -> Vec<playwright_tests::TestOccurrence<PlaywrightSelector>> {
    let mut visitor = PlaywrightSelectorVisitor {
        source,
        regexes,
        test_id_attributes,
        status: playwright_tests::TestStatus::Active,
        annotation_status: playwright_tests::TestStatus::Active,
        selectors: Vec::new(),
        current_test_name: None,
        describe_stack: Vec::new(),
    };
    visitor.visit_program(program);
    visitor.selectors.sort();
    visitor.selectors.dedup();
    visitor.selectors
}

struct PlaywrightSelectorVisitor<'a, 'r> {
    source: &'a str,
    regexes: &'r SelectorRegexes,
    test_id_attributes: &'r [String],
    status: playwright_tests::TestStatus,
    annotation_status: playwright_tests::TestStatus,
    selectors: Vec<playwright_tests::TestOccurrence<PlaywrightSelector>>,
    current_test_name: Option<String>,
    describe_stack: Vec<String>,
}

impl<'a> oxc_ast_visit::Visit<'a> for PlaywrightSelectorVisitor<'a, '_> {
    fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
        if callee_is_static_member_named(&call.callee, "getByTestId") {
            extract_get_by_test_id_call(
                call,
                self.source,
                self.test_id_attributes,
                &mut |selector| self.insert(selector),
            );
        } else if let Some(argument_mode) = selector_argument_mode(&call.callee) {
            for selector in selector_argument_literals(call, self.source, argument_mode) {
                extract_css_attribute_selectors(
                    &selector,
                    &self.regexes.playwright_attributes,
                    &mut |selector| self.insert(selector),
                );
                if self.regexes.html_ids {
                    extract_css_id_selectors(&selector, &mut |selector| self.insert(selector));
                }
            }
        }

        if let Some(describe) = playwright_tests::describe_name(call) {
            let traversal = playwright_tests::test_callback_traversal(call, self.annotation_status);
            if let Some((callback_index, callback_status)) = traversal {
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
                return;
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
            oxc_ast_visit::walk::walk_call_expression(self, call);
            return;
        }

        let (callback_index, callback_status) = traversal.expect("checked traversal");
        let test_name = playwright_tests::test_callback_identity(call);
        let previous_test_name = self.current_test_name.clone();
        if test_name.is_some() {
            self.current_test_name = test_name;
        }
        for (index, argument) in call.arguments.iter().enumerate() {
            if index == callback_index {
                self.with_status(callback_status, |visitor| {
                    visitor.with_annotation_scope(|visitor| visitor.visit_argument(argument));
                });
            } else {
                self.visit_argument(argument);
            }
        }
        self.current_test_name = previous_test_name;
    }

    fn visit_if_statement(&mut self, statement: &oxc_ast::ast::IfStatement<'a>) {
        self.visit_expression(&statement.test);
        let status = playwright_tests::status_for_if_branch(self.status);
        self.with_status(status, |visitor| {
            visitor.visit_statement(&statement.consequent);
            if let Some(alternate) = &statement.alternate {
                visitor.visit_statement(alternate);
            }
        });
    }

    fn visit_conditional_expression(
        &mut self,
        expression: &oxc_ast::ast::ConditionalExpression<'a>,
    ) {
        self.visit_expression(&expression.test);
        let status = playwright_tests::status_for_if_branch(self.status);
        self.with_status(status, |visitor| {
            visitor.visit_expression(&expression.consequent);
            visitor.visit_expression(&expression.alternate);
        });
    }

    fn visit_logical_expression(&mut self, expression: &oxc_ast::ast::LogicalExpression<'a>) {
        self.visit_expression(&expression.left);
        let status = playwright_tests::status_for_if_branch(self.status);
        self.with_status(status, |visitor| {
            visitor.visit_expression(&expression.right)
        });
    }
}

impl PlaywrightSelectorVisitor<'_, '_> {
    fn insert(&mut self, value: PlaywrightSelector) {
        self.selectors.push(playwright_tests::TestOccurrence {
            value,
            status: self.status.merge(self.annotation_status),
            test_name: self.current_test_name.clone(),
            describe_path: self.describe_stack.clone(),
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

    fn apply_annotation_call(&mut self, call: &oxc_ast::ast::CallExpression<'_>) {
        if let Some(status) = playwright_tests::annotation_status_for_call(call) {
            let status = playwright_tests::merge_annotation_status(self.status, status);
            self.annotation_status =
                playwright_tests::merge_annotation_status(self.annotation_status, status);
        }
    }
}
