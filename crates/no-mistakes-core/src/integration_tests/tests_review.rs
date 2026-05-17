use super::*;
use oxc_ast_visit::{walk, Visit};
use std::path::PathBuf;

fn fixture_file(file: &str) -> PathBuf {
    crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/integration-tests/coverage")
            .join(file),
    )
}

#[test]
fn skipped_describe_helpers_are_detected_without_walking_nested_functions() {
    let path = fixture_file("src/calls.ts");
    let source = std::fs::read_to_string(&path).unwrap();
    crate::ast::with_program(&path, &source, |program, _| {
        let mut assertions = ReviewCallAssertions::default();
        assertions.visit_program(program);
        assert!(assertions.saw_skipped_describe);
        assert!(assertions.saw_nested_function_call_ignored);
    })
    .unwrap();
}

#[derive(Default)]
struct ReviewCallAssertions {
    saw_skipped_describe: bool,
    saw_nested_function_call_ignored: bool,
}

impl<'a> Visit<'a> for ReviewCallAssertions {
    fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
        let path = crate::ast::expression_path(&call.callee);
        if path
            .as_ref()
            .is_some_and(|path| path == &["describe", "skip"])
        {
            self.saw_skipped_describe =
                calls::describe_name(call).is_none() && calls::is_skipped_describe(call);
        }
        if calls::test_name(call).as_deref() == Some("function callback") {
            let (argument, _) = calls::callback_argument(call).unwrap();
            let calls = calls::collect_calls(argument);
            self.saw_nested_function_call_ignored = !format!("{calls:?}").contains("nestedOnly");
        }
        walk::walk_call_expression(self, call);
    }
}
