use super::*;
use oxc_ast_visit::Visit;
use std::collections::HashSet;
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

#[test]
fn namespace_import_is_tracked_as_shadowed() {
    let source = "import * as Fetcher from './fetcher';\nFetcher.fetch('/api/hidden');";
    crate::ast::with_program(Path::new("fixture.ts"), source, |program, source| {
        let mut visitor = FetchVisitor::new(source, "fixture.ts", false, false);
        visitor.visit_program(program);
        // The namespace import shadows the identifier 'Fetcher'
        assert!(visitor.fetch_scope_stack[0]
            .shadowed_identifiers
            .contains("Fetcher"));
    })
    .unwrap();
}

#[test]
fn fetch_call_outside_component_span_is_excluded() {
    // Set a component_span that doesn't include the fetch call position.
    // The fetch call "fetch('/api/outside')" starts at a position after the span.
    let source = "fetch('/api/outside');";
    crate::ast::with_program(Path::new("fixture.ts"), source, |program, source| {
        let mut visitor = FetchVisitor::new(source, "fixture.ts", false, false);
        // Set component_span to [0, 0) — zero-length span so the fetch is outside it
        visitor.component_span = Some(oxc_span::Span::new(0, 0));
        visitor.visit_program(program);
        assert!(
            visitor.fetches.is_empty(),
            "fetch outside component span should be excluded"
        );
    })
    .unwrap();
}

#[test]
fn fetch_call_before_component_span_is_excluded() {
    let source = "fetch('/api/before');";
    crate::ast::with_program(Path::new("fixture.ts"), source, |program, source| {
        let mut visitor = FetchVisitor::new(source, "fixture.ts", false, false);
        visitor.component_span = Some(oxc_span::Span::new(100, 120));
        visitor.visit_program(program);
        assert!(visitor.fetches.is_empty());
    })
    .unwrap();
}

#[test]
fn mark_identifier_shadowed_in_var_scope_falls_back_when_no_var_scope_exists() {
    let mut visitor = FetchVisitor::new("", "fixture.ts", false, false);
    // Replace the base scope (which has tracks_var_bindings=true) with one that has false.
    // This exercises the fallback path at the end of mark_identifier_shadowed_in_var_scope.
    visitor.fetch_scope_stack = vec![FetchScope {
        shadowed_identifiers: HashSet::new(),
        tracks_var_bindings: false,
    }];
    visitor.mark_identifier_shadowed_in_var_scope("fetch");
    assert!(visitor
        .fetch_scope_stack
        .last()
        .unwrap()
        .shadowed_identifiers
        .contains("fetch"));
}
