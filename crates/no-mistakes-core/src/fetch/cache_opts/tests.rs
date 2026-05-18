use super::*;
use oxc_ast::ast::CallExpression;
use oxc_ast_visit::{walk, Visit};
use std::path::Path;

struct CallExpressionVisitor<'a> {
    found_expr: Option<&'a CallExpression<'a>>,
}

impl<'a> Visit<'a> for CallExpressionVisitor<'a> {
    fn visit_call_expression(&mut self, expr: &CallExpression<'a>) {
        if self.found_expr.is_none() {
            // Unsafe lifecycle bypass for test purpose since we just want the first one
            self.found_expr = Some(unsafe {
                std::mem::transmute::<
                    &oxc_ast::ast::CallExpression<'_>,
                    &oxc_ast::ast::CallExpression<'_>,
                >(expr)
            });
        }
        walk::walk_call_expression(self, expr);
    }
}

fn with_call_expression<F>(source: &str, f: F)
where
    F: FnOnce(&CallExpression<'_>),
{
    crate::ast::with_program(Path::new("test.ts"), source, |program, _| {
        let mut visitor = CallExpressionVisitor { found_expr: None };
        visitor.visit_program(program);
        f(visitor
            .found_expr
            .expect("Expected to find a call expression"));
    })
    .unwrap();
}

#[test]
fn test_cache_wrapper_name_react_cache() {
    with_call_expression("cache(fn)", |expr| {
        let result = cache_wrapper_name(expr);
        assert!(result.is_some());
        let (name, kind) = result.unwrap();
        assert_eq!(name, "cache");
        assert_eq!(kind, CacheKind::ReactCache);
    });
}

#[test]
fn test_cache_wrapper_name_unstable_cache() {
    with_call_expression("unstable_cache(fn)", |expr| {
        let result = cache_wrapper_name(expr);
        assert!(result.is_some());
        let (name, kind) = result.unwrap();
        assert_eq!(name, "unstable_cache");
        assert_eq!(kind, CacheKind::UnstableCache);
    });
}

#[test]
fn test_cache_wrapper_name_other_function() {
    with_call_expression("other_function(fn)", |expr| {
        let result = cache_wrapper_name(expr);
        assert!(result.is_none());
    });
}

#[test]
fn test_cache_wrapper_name_member_expression() {
    with_call_expression("obj.cache(fn)", |expr| {
        let result = cache_wrapper_name(expr);
        assert!(result.is_none());
    });
}
