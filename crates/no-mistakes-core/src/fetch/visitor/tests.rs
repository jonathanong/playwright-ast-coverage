use super::*;
use oxc_ast_visit::Visit;
use std::path::Path;

#[test]
fn function_declarations_shadow_global_fetch() {
    let source = "function fetch() {}\nfetch('/api/hidden');";
    crate::ast::with_program(Path::new("fixture.ts"), source, |program, source| {
        let mut visitor = FetchVisitor::new(source, "fixture.ts", false, false);
        visitor.visit_program(program);
        assert!(visitor.fetches.is_empty());
    })
    .unwrap();
}

#[test]
fn is_fetch_shadowed_state_method() {
    let mut visitor = FetchVisitor::new("", "fixture.ts", false, false);

    // Test initial state
    assert!(!visitor.is_fetch_shadowed());

    // Test mark_fetch_shadowed
    visitor.mark_fetch_shadowed();
    assert!(visitor.is_fetch_shadowed());

    // Test scope entry/exit interactions
    visitor.enter_fetch_scope(false);
    assert!(visitor.is_fetch_shadowed());

    visitor.leave_fetch_scope();
    assert!(visitor.is_fetch_shadowed());
}

#[test]
fn is_fetch_shadowed_respects_scope_stack() {
    let mut visitor = FetchVisitor::new("", "fixture.ts", false, false);

    // Initially, fetch is not shadowed
    assert!(!visitor.is_fetch_shadowed());

    // Shadowing another identifier should not affect fetch tracking
    visitor.mark_identifier_shadowed("not_fetch");
    assert!(!visitor.is_fetch_shadowed());

    // Enter a new scope
    visitor.enter_fetch_scope(true);
    assert!(!visitor.is_fetch_shadowed());

    // Shadow fetch in the inner scope
    visitor.mark_fetch_shadowed();
    assert!(visitor.is_fetch_shadowed());

    // Enter another scope, fetch is still shadowed because of the outer scope
    visitor.enter_fetch_scope(false);
    assert!(visitor.is_fetch_shadowed());

    // Leave the innermost scope, fetch is still shadowed
    visitor.leave_fetch_scope();
    assert!(visitor.is_fetch_shadowed());

    // Leave the scope where fetch was shadowed, now it's no longer shadowed
    visitor.leave_fetch_scope();
    assert!(!visitor.is_fetch_shadowed());
    assert_eq!(visitor.fetch_scope_stack.len(), 1);
}

#[test]
fn anonymous_default_function_declaration_keeps_fetch_visible_inside_body() {
    let source = "export default function() { fetch('/api/visible'); }";
    crate::ast::with_program(Path::new("fixture.ts"), source, |program, source| {
        let mut visitor = FetchVisitor::new(source, "fixture.ts", false, false);
        visitor.visit_program(program);
        assert_eq!(visitor.fetches.len(), 1);
    })
    .unwrap();
}
