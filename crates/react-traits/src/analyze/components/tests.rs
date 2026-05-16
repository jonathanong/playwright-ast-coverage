use super::{extract_components, is_component_expr};
use no_mistakes_core::ast;

fn check_names(source: &str) -> Vec<String> {
    let path = std::path::Path::new("test.tsx");
    ast::with_program(path, source, |program, _| {
        extract_components(program)
            .into_iter()
            .map(|c| c.name)
            .collect::<Vec<_>>()
    })
    .unwrap()
}

fn check_is_expr(source: &str) -> bool {
    let path = std::path::Path::new("test.tsx");
    ast::with_program(path, source, |program, _| {
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
    let names = check_names("export default function Foo() { return <div/>; }");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_arrow() {
    let names = check_names("export default () => <div/>;");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_memo_call() {
    let names = check_names("export default memo(function Foo() { return <div/>; });");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_react_memo_call() {
    let names = check_names("export default React.memo(function Foo() { return <div/>; });");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_forwardref() {
    let names =
        check_names("export default forwardRef(function Foo(props, ref) { return <div/>; });");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_lazy() {
    let names = check_names("export default lazy(() => import('./Foo'));");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_class() {
    let names = check_names("export default class Foo {}");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_function_expression() {
    let names = check_names("export default function() { return <div/>; }");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_named_function() {
    let names = check_names("export function Foo() { return <div/>; }");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn export_named_function_lowercase_ignored() {
    let names = check_names("export function foo() { return <div/>; }");
    assert!(names.is_empty());
}

#[test]
fn export_const_arrow() {
    let names = check_names("export const Foo = () => <div/>;");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn export_const_arrow_lowercase_ignored() {
    let names = check_names("export const foo = () => <div/>;");
    assert!(names.is_empty());
}

#[test]
fn export_const_memo_wrapped() {
    let names = check_names("export const Foo = memo(() => <div/>);");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn no_components_empty_file() {
    let names = check_names("");
    assert!(names.is_empty());
}

#[test]
fn no_components_non_export() {
    let names = check_names("function foo() { return <div/>; }");
    assert!(names.is_empty());
}

#[test]
fn is_component_expr_arrow() {
    assert!(check_is_expr("() => <div/>"));
}

#[test]
fn is_component_expr_call_not_memo() {
    assert!(!check_is_expr("someOtherFn()"));
}

#[test]
fn is_component_expr_literal_false() {
    assert!(!check_is_expr("42"));
}

#[test]
fn is_component_expr_call_unknown_callee_false() {
    // The callee is a computed member expression (neither Identifier nor StaticMember),
    // which hits the `_ => return false` branch.
    assert!(!check_is_expr("obj[key]()"));
}

#[test]
fn is_component_expr_parenthesized_arrow() {
    // Parenthesized arrow function exercises the ParenthesizedExpression branch.
    let path = std::path::Path::new("test.tsx");
    let source = "export default (() => <div/>);";
    let names: Vec<String> = ast::with_program(path, source, |program, _| {
        super::extract_components(program)
            .into_iter()
            .map(|c| c.name)
            .collect()
    })
    .unwrap();
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_call_unknown_callee_ignored() {
    // A call expression with a computed/unknown callee should not produce a component.
    let names = check_names("export default obj[key]();");
    assert!(names.is_empty());
}

#[test]
fn export_named_variable_non_component_expr_ignored() {
    // `export const Foo = 42;` — Foo is uppercase but init is not a component expr.
    let names = check_names("export const Foo = 42;");
    assert!(names.is_empty());
}

#[test]
fn export_named_other_declaration_ignored() {
    // Class declarations in named exports are not handled.
    let names = check_names("export class Foo {}");
    assert!(names.is_empty());
}

#[test]
fn export_default_string_literal_ignored() {
    // String literal as default export hits the _ => {} fallthrough.
    let names = check_names("export default 'hello';");
    assert!(names.is_empty());
}

#[test]
fn export_const_react_memo_wrapped() {
    // React.memo(() => <div/>) — StaticMemberExpression callee in is_component_expr (line 119).
    let names = check_names("export const Foo = React.memo(() => <div/>);");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn export_named_reexport_no_declaration() {
    // `export { Foo }` — ExportNamedDeclaration with declaration = None, exercises line 101.
    let names = check_names("export { Foo };");
    assert!(names.is_empty());
}

#[test]
fn export_named_destructured_variable() {
    // Array destructuring binding — BindingPattern is ArrayPattern not BindingIdentifier,
    // exercises the fall-through in the if-let BindingIdentifier match (line 96).
    let names = check_names("export const [a, b] = [1, 2];");
    assert!(names.is_empty());
}

#[test]
fn export_named_let_no_init() {
    // Variable declared with no init — exercises the `if let Some(init)` None path (line 94).
    let names = check_names("export let Foo;");
    assert!(names.is_empty());
}

#[test]
fn export_default_identifier_resolves_local_component() {
    // Common Next.js pattern: const Page = () => ...; export default Page
    let names = check_names("const Page = () => <div/>;\nexport default Page;");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_identifier_unknown_name_ignored() {
    // `export default Foo` where Foo was not seen as a component const — ignored
    let names = check_names("export default Foo;");
    assert!(names.is_empty());
}

#[test]
fn is_component_expr_parenthesized_wraps_arrow() {
    // ((() => <div/>)) — double-parenthesized arrow triggers ParenthesizedExpression (line 124).
    let path = std::path::Path::new("test.tsx");
    let source = "export const Foo = ((() => <div/>));";
    let names: Vec<String> = ast::with_program(path, source, |program, _| {
        super::extract_components(program)
            .into_iter()
            .map(|c| c.name)
            .collect()
    })
    .unwrap();
    assert_eq!(names, vec!["Foo"]);
}
