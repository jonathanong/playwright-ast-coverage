use crate::ast;
use oxc_ast::ast::{Argument, CallExpression, Expression};

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
        if self.rank() >= other.rank() {
            self
        } else {
            other
        }
    }

    fn rank(self) -> u8 {
        match self {
            TestStatus::Active => 0,
            TestStatus::Conditional => 1,
            TestStatus::Skipped => 2,
        }
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
    if callee_contains_playwright_skip_if(&call.callee) {
        return Some(TestStatus::Conditional);
    }

    let path = ast::expression_path(&call.callee)?;
    if !is_playwright_test_path(&path) {
        return None;
    }

    if is_non_runnable_path(&path) {
        return non_runnable_callback_status(call, &path);
    }

    Some(TestStatus::Active)
}

pub fn test_callback_traversal(
    call: &CallExpression<'_>,
    annotation_status: TestStatus,
) -> Option<(usize, TestStatus)> {
    Some((
        callback_argument_index(call)?,
        test_callback_status(call)?.merge(annotation_status),
    ))
}

pub fn annotation_status_for_call(call: &CallExpression<'_>) -> Option<TestStatus> {
    if callee_contains_playwright_skip_if(&call.callee) {
        return Some(TestStatus::Conditional);
    }

    let path = ast::expression_path(&call.callee)?;
    if !is_non_runnable_path(&path) {
        return None;
    }

    non_runnable_annotation_status(call, &path)
}

pub fn status_for_if_branch(current: TestStatus) -> TestStatus {
    current.merge(TestStatus::Conditional)
}

pub fn merge_annotation_status(context: TestStatus, annotation: TestStatus) -> TestStatus {
    if context == TestStatus::Conditional && annotation == TestStatus::Skipped {
        TestStatus::Conditional
    } else {
        context.merge(annotation)
    }
}

fn non_runnable_callback_status(call: &CallExpression<'_>, path: &[String]) -> Option<TestStatus> {
    let Some(first) = call.arguments.first() else {
        return Some(TestStatus::Skipped);
    };

    match first {
        Argument::BooleanLiteral(value) if value.value => Some(TestStatus::Skipped),
        Argument::BooleanLiteral(_) => None,
        Argument::StringLiteral(_) | Argument::TemplateLiteral(_) => Some(TestStatus::Skipped),
        argument
            if argument_is_function(argument) && path.iter().any(|part| part == "describe") =>
        {
            Some(TestStatus::Skipped)
        }
        argument if argument_is_function(argument) => None,
        _ => Some(TestStatus::Conditional),
    }
}

fn non_runnable_annotation_status(
    call: &CallExpression<'_>,
    path: &[String],
) -> Option<TestStatus> {
    if path.iter().any(|part| part == "describe") && callback_argument_index(call).is_some() {
        return None;
    }

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

fn is_non_runnable_path(parts: &[String]) -> bool {
    matches!(parts.first().map(String::as_str), Some("test"))
        && parts.iter().any(|part| part == "skip" || part == "fixme")
}

fn callee_contains_playwright_skip_if(expression: &Expression<'_>) -> bool {
    if ast::expression_path(expression).is_some_and(|parts| is_playwright_skip_if_path(&parts)) {
        return true;
    }

    match expression {
        Expression::CallExpression(call) => callee_contains_playwright_skip_if(&call.callee),
        Expression::ParenthesizedExpression(parenthesized) => {
            callee_contains_playwright_skip_if(&parenthesized.expression)
        }
        _ => false,
    }
}

fn is_playwright_skip_if_path(parts: &[String]) -> bool {
    matches!(parts.first().map(String::as_str), Some("test"))
        && parts.iter().any(|part| part == "skipIf")
}

fn argument_is_function(argument: &Argument<'_>) -> bool {
    matches!(
        argument,
        Argument::ArrowFunctionExpression(_) | Argument::FunctionExpression(_)
    )
}

pub fn test_callback_identity(call: &CallExpression<'_>) -> Option<String> {
    let path = ast::expression_path(&call.callee)?;
    if !is_playwright_test_path(&path) {
        return None;
    }
    call.arguments.first().and_then(first_string_arg)
}

fn first_string_arg(arg: &Argument<'_>) -> Option<String> {
    match arg {
        Argument::StringLiteral(s) => Some(s.value.to_string()),
        Argument::TemplateLiteral(t) if t.expressions.is_empty() => {
            t.quasis.first().map(|q| {
                q.value
                    .cooked
                    .as_ref()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| q.value.raw.to_string())
            })
        }
        _ => None,
    }
}

pub fn describe_name(call: &CallExpression<'_>) -> Option<String> {
    let path = ast::expression_path(&call.callee)?;
    let is_describe = matches!(path.first().map(String::as_str), Some("test"))
        && path.iter().any(|part| part == "describe");
    if !is_describe {
        return None;
    }
    call.arguments.first().and_then(first_string_arg)
}
