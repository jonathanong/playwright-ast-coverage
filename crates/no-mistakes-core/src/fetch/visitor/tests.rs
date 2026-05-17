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
fn anonymous_default_function_declaration_keeps_fetch_visible_inside_body() {
    let source = "export default function() { fetch('/api/visible'); }";
    crate::ast::with_program(Path::new("fixture.ts"), source, |program, source| {
        let mut visitor = FetchVisitor::new(source, "fixture.ts", false, false);
        visitor.visit_program(program);
        assert_eq!(visitor.fetches.len(), 1);
    })
    .unwrap();
}
