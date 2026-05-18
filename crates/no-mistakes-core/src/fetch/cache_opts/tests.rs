use super::*;
use crate::ast::with_program;
use oxc_ast::ast::{CallExpression, Expression, Statement};
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
fn test_cache_wrapper_name_iife() {
    with_call_expression("(function() {})(fn)", |expr| {
        let result = cache_wrapper_name(expr);
        assert!(result.is_none());
    });
}

#[test]
fn test_cache_wrapper_name_parenthesized_identifier() {
    with_call_expression("(cache)(fn)", |expr| {
        let result = cache_wrapper_name(expr);
        assert!(result.is_none());
    });
}

#[test]
fn test_cache_wrapper_name_nested_call() {
    with_call_expression("foo(fn)()", |expr| {
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

fn extract_from_source(source: &str) -> (bool, CacheKind) {
    let mut result = None;
    with_program(Path::new("test.ts"), source, |program, _| {
        for stmt in &program.body {
            if let Statement::ExpressionStatement(expr_stmt) = stmt {
                if let Expression::CallExpression(call_expr) = &expr_stmt.expression {
                    if let Some(arg) = call_expr.arguments.get(1) {
                        if let Some(Expression::ObjectExpression(obj)) = arg.as_expression() {
                            result = Some(extract_fetch_cache_options(obj));
                            return;
                        }
                    }
                }
            }
        }
    })
    .unwrap();

    result.expect("expected fetch(...) call with second argument object")
}

#[test]
fn test_cache_force_cache() {
    let source = "fetch('url', { cache: 'force-cache' });";
    let (cached, kind) = extract_from_source(source);
    assert!(cached);
    assert_eq!(kind, CacheKind::FetchCache);
}

#[test]
fn test_next_revalidate() {
    let source = "fetch('url', { next: { revalidate: 60 } });";
    let (cached, kind) = extract_from_source(source);
    assert!(cached);
    assert_eq!(kind, CacheKind::FetchNextRevalidate);
}

#[test]
fn test_next_tags() {
    let source = "fetch('url', { next: { tags: [] } });";
    let (cached, kind) = extract_from_source(source);
    assert!(cached);
    assert_eq!(kind, CacheKind::FetchNextTags);
}

#[test]
fn test_empty_options() {
    let source = "fetch('url', {});";
    let (cached, kind) = extract_from_source(source);
    assert!(!cached);
    assert_eq!(kind, CacheKind::None);
}

#[test]
fn test_other_options() {
    let source = "fetch('url', { method: 'POST', cache: 'no-store' });";
    let (cached, kind) = extract_from_source(source);
    assert!(!cached);
    assert_eq!(kind, CacheKind::None);
}

#[test]
fn test_next_revalidate_zero() {
    let source = "fetch('url', { next: { revalidate: 0 } });";
    let (cached, kind) = extract_from_source(source);
    assert!(!cached);
    assert_eq!(kind, CacheKind::None);
}

#[test]
fn test_next_empty() {
    let source = "fetch('url', { next: {} });";
    let (cached, kind) = extract_from_source(source);
    assert!(!cached);
    assert_eq!(kind, CacheKind::None);
}

#[test]
fn test_spread_options() {
    let source = "fetch('url', { ...spreadOpts });";
    let (cached, kind) = extract_from_source(source);
    assert!(!cached);
    assert_eq!(kind, CacheKind::None);
}

#[test]
fn test_dynamic_cache_key_is_static_string() {
    let source = "fetch('url', { ['cache']: 'force-cache' });";
    let (cached, kind) = extract_from_source(source);
    assert!(cached);
    assert_eq!(kind, CacheKind::FetchCache);
}

#[test]
fn test_cache_not_string() {
    let source = "fetch('url', { cache: true });";
    let (cached, kind) = extract_from_source(source);
    assert!(!cached);
    assert_eq!(kind, CacheKind::None);
}

#[test]
fn test_cache_unknown_mode() {
    let source = "fetch('url', { cache: 'unknown-mode' });";
    let (cached, kind) = extract_from_source(source);
    assert!(!cached);
    assert_eq!(kind, CacheKind::None);
}

#[test]
fn test_next_not_object() {
    let source = "fetch('url', { next: true });";
    let (cached, kind) = extract_from_source(source);
    assert!(!cached);
    assert_eq!(kind, CacheKind::None);
}

#[test]
fn test_next_is_null() {
    let source = "fetch('url', { next: null });";
    let (cached, kind) = extract_from_source(source);
    assert!(!cached);
    assert_eq!(kind, CacheKind::None);
}

#[test]
fn test_next_revalidate_string_value() {
    let source = "fetch('url', { next: { revalidate: '60' } });";
    let (cached, kind) = extract_from_source(source);
    assert!(!cached);
    assert_eq!(kind, CacheKind::None);
}

#[test]
fn test_next_tags_non_array() {
    let source = "fetch('url', { next: { tags: 'foo' } });";
    let (cached, kind) = extract_from_source(source);
    assert!(!cached);
    assert_eq!(kind, CacheKind::None);
}

#[test]
fn test_next_spread() {
    let source = "fetch('url', { next: { ...spread } });";
    let (cached, kind) = extract_from_source(source);
    assert!(!cached);
    assert_eq!(kind, CacheKind::None);
}

#[test]
fn test_next_dynamic_key_is_static_string() {
    let source = "fetch('url', { next: { ['revalidate']: 60 } });";
    let (cached, kind) = extract_from_source(source);
    assert!(cached);
    assert_eq!(kind, CacheKind::FetchNextRevalidate);
}

#[test]
fn test_next_unrelated_key() {
    let source = "fetch('url', { next: { unrelated: 1 } });";
    let (cached, kind) = extract_from_source(source);
    assert!(!cached);
    assert_eq!(kind, CacheKind::None);
}

#[test]
fn test_cache_and_next_properties() {
    let source = "fetch('url', { cache: 'force-cache', next: { revalidate: 60 } });";
    let (cached, kind) = extract_from_source(source);
    assert!(cached);
    assert_eq!(kind, CacheKind::FetchNextRevalidate);
}
