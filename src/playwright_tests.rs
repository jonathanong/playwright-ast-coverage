use crate::ast;
use oxc_ast::ast::{Argument, CallExpression, Expression, FunctionBody, IfStatement, Program};
use oxc_ast_visit::{walk, Visit};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TestStatus {
    Active,
    Conditional,
    Skipped,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TestOccurrence<T> {
    pub value: T,
    pub status: TestStatus,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TestPolicy {
    pub assert_conditional_tests: bool,
    pub allow_skipped_tests: bool,
}

impl TestStatus {
    pub fn merge(self, other: Self) -> Self {
        self.max(other)
    }
}

impl TestPolicy {
    pub fn allows(self, status: TestStatus) -> bool {
        match status {
            TestStatus::Active => true,
            TestStatus::Conditional => !self.assert_conditional_tests,
            TestStatus::Skipped => self.allow_skipped_tests,
        }
    }
}

pub fn callback_argument_index(call: &CallExpression<'_>) -> Option<usize> {
    call.arguments.iter().rposition(argument_is_function)
}

pub fn test_callback_status(call: &CallExpression<'_>) -> Option<TestStatus> {
    callback_argument_index(call)?;
    if callee_contains_skip_if(&call.callee) {
        return Some(TestStatus::Conditional);
    }

    let path = ast::expression_path(&call.callee)?;
    if !is_playwright_test_path(&path) {
        return None;
    }

    if is_skip_path(&path) {
        return Some(skip_call_status(call).unwrap_or(TestStatus::Active));
    }

    Some(TestStatus::Active)
}

pub fn function_argument_annotation_status(argument: &Argument<'_>) -> Option<TestStatus> {
    match argument {
        Argument::ArrowFunctionExpression(arrow) => annotation_status_for_body(&arrow.body),
        Argument::FunctionExpression(function) => function
            .body
            .as_ref()
            .and_then(|body| annotation_status_for_body(body)),
        _ => None,
    }
}

pub fn annotation_status_for_call(call: &CallExpression<'_>) -> Option<TestStatus> {
    if callee_contains_skip_if(&call.callee) {
        return Some(TestStatus::Conditional);
    }

    let path = ast::expression_path(&call.callee)?;
    if !is_skip_path(&path) {
        return None;
    }

    skip_call_status(call)
}

pub fn status_for_if_branch(current: TestStatus) -> TestStatus {
    current.merge(TestStatus::Conditional)
}

fn annotation_status_for_body(body: &FunctionBody<'_>) -> Option<TestStatus> {
    let mut visitor = AnnotationVisitor {
        status: TestStatus::Active,
        annotations: BTreeSet::new(),
    };
    visitor.visit_function_body(body);
    visitor.annotations.into_iter().max()
}

fn skip_call_status(call: &CallExpression<'_>) -> Option<TestStatus> {
    let Some(first) = call.arguments.first() else {
        return Some(TestStatus::Skipped);
    };

    match first {
        Argument::BooleanLiteral(value) if value.value => Some(TestStatus::Skipped),
        Argument::BooleanLiteral(_) => None,
        Argument::StringLiteral(_) | Argument::TemplateLiteral(_) => Some(TestStatus::Skipped),
        _ => Some(TestStatus::Conditional),
    }
}

fn is_playwright_test_path(parts: &[String]) -> bool {
    matches!(parts.first().map(String::as_str), Some("test"))
        && (parts.len() == 1
            || parts.iter().any(|part| part == "describe")
            || parts.iter().any(|part| {
                matches!(
                    part.as_str(),
                    "only" | "skip" | "fixme" | "slow" | "serial" | "parallel"
                )
            }))
}

fn is_skip_path(parts: &[String]) -> bool {
    matches!(parts.first().map(String::as_str), Some("test"))
        && parts.iter().any(|part| part == "skip")
}

fn callee_contains_skip_if(expression: &Expression<'_>) -> bool {
    match expression {
        Expression::Identifier(identifier) => identifier.name == "skipIf",
        Expression::StaticMemberExpression(member) => {
            member.property.name == "skipIf" || callee_contains_skip_if(&member.object)
        }
        Expression::CallExpression(call) => callee_contains_skip_if(&call.callee),
        Expression::ParenthesizedExpression(parenthesized) => {
            callee_contains_skip_if(&parenthesized.expression)
        }
        _ => false,
    }
}

fn argument_is_function(argument: &Argument<'_>) -> bool {
    matches!(
        argument,
        Argument::ArrowFunctionExpression(_) | Argument::FunctionExpression(_)
    )
}

struct AnnotationVisitor {
    status: TestStatus,
    annotations: BTreeSet<TestStatus>,
}

impl<'a> Visit<'a> for AnnotationVisitor {
    fn visit_program(&mut self, program: &Program<'a>) {
        walk::walk_program(self, program);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Some(callback_index) = callback_argument_index(call) {
            self.visit_expression(&call.callee);
            for (index, argument) in call.arguments.iter().enumerate() {
                if index != callback_index {
                    self.visit_argument(argument);
                }
            }
        } else {
            if let Some(status) = annotation_status_for_call(call) {
                self.annotations
                    .insert(merge_annotation_status(self.status, status));
            }
            walk::walk_call_expression(self, call);
        }
    }

    fn visit_if_statement(&mut self, statement: &IfStatement<'a>) {
        self.visit_expression(&statement.test);
        let previous = self.status;
        self.status = status_for_if_branch(previous);
        self.visit_statement(&statement.consequent);
        if let Some(alternate) = &statement.alternate {
            self.visit_statement(alternate);
        }
        self.status = previous;
    }
}

fn merge_annotation_status(context: TestStatus, annotation: TestStatus) -> TestStatus {
    if context == TestStatus::Conditional && annotation == TestStatus::Skipped {
        TestStatus::Conditional
    } else {
        context.merge(annotation)
    }
}
