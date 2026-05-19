use super::{extract_components, is_component_expr};
use crate::ast;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/react-traits-analyze/components")
        .join(name)
}

fn load_fixture(name: &str) -> (PathBuf, String) {
    let path = fixture(name).join("test.tsx");
    let path = if path.exists() {
        path
    } else {
        fixture(name).join("test.ts")
    };
    let source = std::fs::read_to_string(&path).expect("fixture must be readable");
    (path, source)
}

fn check_names(name: &str) -> Vec<String> {
    let (path, source) = load_fixture(name);
    ast::with_program(&path, &source, |program, _| {
        extract_components(program)
            .into_iter()
            .map(|c| c.name)
            .collect::<Vec<_>>()
    })
    .unwrap()
}

fn check_is_expr(name: &str) -> bool {
    let (path, source) = load_fixture(name);
    ast::with_program(&path, &source, |program, _| {
        let stmt = program.body.first().expect("expected statement");
        let oxc_ast::ast::Statement::ExpressionStatement(expr_stmt) = stmt else {
            panic!("expected expression statement");
        };
        is_component_expr(&expr_stmt.expression)
    })
    .unwrap()
}

#[test]
fn export_default_function() {
    let names = check_names("export-default-function");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_arrow() {
    let names = check_names("export-default-arrow");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_memo_call() {
    let names = check_names("export-default-memo-call");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_react_memo_call() {
    let names = check_names("export-default-react-memo-call");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_forwardref() {
    let names = check_names("export-default-forwardref");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_lazy() {
    let names = check_names("export-default-lazy");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_class() {
    let names = check_names("export-default-class");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_function_expression() {
    let names = check_names("export-default-function-expression");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_named_function() {
    let names = check_names("export-named-function");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn export_named_function_lowercase_ignored() {
    let names = check_names("export-named-function-lowercase");
    assert!(names.is_empty());
}

#[test]
fn export_const_arrow() {
    let names = check_names("export-const-arrow");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn export_const_arrow_lowercase_ignored() {
    let names = check_names("export-const-arrow-lowercase");
    assert!(names.is_empty());
}

#[test]
fn export_const_memo_wrapped() {
    let names = check_names("export-const-memo-wrapped");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn no_components_empty_file() {
    let names = check_names("no-components-empty");
    assert!(names.is_empty());
}

#[test]
fn no_components_non_export() {
    let names = check_names("no-components-non-export");
    assert!(names.is_empty());
}

#[test]
fn is_component_expr_arrow() {
    assert!(check_is_expr("is-component-expr-arrow"));
}

#[test]
fn is_component_expr_call_not_memo() {
    assert!(!check_is_expr("is-component-expr-call-not-memo"));
}

#[test]
fn is_component_expr_literal_false() {
    assert!(!check_is_expr("is-component-expr-literal"));
}

#[test]
fn is_component_expr_call_unknown_callee_false() {
    assert!(!check_is_expr("is-component-expr-computed-callee"));
}

#[test]
fn is_component_expr_parenthesized_arrow() {
    let names = check_names("is-component-expr-parenthesized-arrow");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_call_unknown_callee_ignored() {
    let names = check_names("export-default-computed-callee");
    assert!(names.is_empty());
}

#[test]
fn export_named_variable_non_component_expr_ignored() {
    let names = check_names("export-named-variable-non-component");
    assert!(names.is_empty());
}

#[test]
fn export_named_other_declaration_ignored() {
    let names = check_names("export-named-other-declaration");
    assert!(names.is_empty());
}

#[test]
fn export_default_string_literal_ignored() {
    let names = check_names("export-default-string-literal");
    assert!(names.is_empty());
}

#[test]
fn export_const_react_memo_wrapped() {
    let names = check_names("export-const-react-memo-wrapped");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn export_named_reexport_no_declaration() {
    let names = check_names("export-named-reexport-no-declaration");
    assert!(names.is_empty());
}

#[test]
fn export_named_destructured_variable() {
    let names = check_names("export-named-destructured-variable");
    assert!(names.is_empty());
}

#[test]
fn export_named_let_no_init() {
    let names = check_names("export-named-let-no-init");
    assert!(names.is_empty());
}

#[test]
fn export_default_identifier_resolves_local_component() {
    let names = check_names("export-default-identifier-resolves-local");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_identifier_unknown_name_ignored() {
    let names = check_names("export-default-identifier-unknown");
    assert!(names.is_empty());
}

#[test]
fn export_list_component() {
    let names = check_names("export-list-component");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn export_list_with_alias() {
    let names = check_names("export-list-with-alias");
    assert_eq!(names, vec!["Bar"]);
}

#[test]
fn export_list_non_component_ignored() {
    let names = check_names("export-list-non-component");
    assert!(names.is_empty());
}

#[test]
fn export_function_declaration_default_export_resolves_default_and_named() {
    let names = check_names("export-function-declaration-default-export");
    assert_eq!(names, vec!["Foo", "default"]);
}

#[test]
fn export_class_declaration_default_export_resolves_default_and_named() {
    let names = check_names("export-class-declaration-default-export");
    assert_eq!(names, vec!["Foo", "default"]);
}

#[test]
fn export_class_extends_component() {
    let names = check_names("export-class-extends-component");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn export_class_extends_react_component() {
    let names = check_names("export-class-extends-react-component");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn export_class_no_superclass_ignored() {
    let names = check_names("export-class-no-superclass");
    assert!(names.is_empty());
}

#[test]
fn export_class_extends_pure_component() {
    let names = check_names("export-class-extends-pure-component");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn local_class_component_resolves_via_export_list() {
    let names = check_names("local-class-component-export-list");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn export_default_function_decl_resolved() {
    let names = check_names("export-default-function-decl-resolved");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_list_resolves_function_decl() {
    let names = check_names("export-list-resolves-function-decl");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn export_default_class_no_superclass_not_component() {
    let names = check_names("export-default-class-no-superclass");
    assert!(names.is_empty());
}

#[test]
fn export_default_class_extends_component_is_component() {
    let names = check_names("export-default-class-extends-component");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_memo_wraps_identifier_uses_decl_span() {
    let names = check_names("export-default-memo-wraps-identifier");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_dynamic_extracted_as_component() {
    let names = check_names("export-default-dynamic");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn is_component_expr_parenthesized_wraps_arrow() {
    let names = check_names("is-component-expr-double-parenthesized-arrow");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn local_var_with_non_component_init_ignored() {
    let names = check_names("local-var-non-component-init");
    assert!(names.is_empty());
}

#[test]
fn local_destructured_var_not_tracked() {
    let names = check_names("local-destructured-var");
    assert!(names.is_empty());
}

#[test]
fn export_default_memo_no_args_is_component() {
    let names = check_names("export-default-memo-no-args");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_named_ts_type_ignored() {
    let names = check_names("export-named-ts-type");
    assert!(names.is_empty());
}

#[test]
fn local_var_no_init_not_tracked() {
    let names = check_names("local-var-no-init");
    assert!(names.is_empty());
}

#[test]
fn export_const_dynamic_wrapped() {
    let names = check_names("export-const-dynamic-wrapped");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn export_class_extends_call_not_component() {
    let names = check_names("export-class-extends-call");
    assert!(names.is_empty());
}

#[test]
fn is_component_expr_non_react_static_member_not_component() {
    assert!(!check_is_expr("is-component-expr-non-react-static-member"));
}

#[test]
fn is_component_expr_react_memo_is_component() {
    assert!(check_is_expr("is-component-expr-react-memo"));
}

#[test]
fn export_default_non_react_member_call_not_component() {
    let names = check_names("export-default-non-react-member-call");
    assert!(names.is_empty());
}
