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
    let names = check_names("export default class Foo extends Component {}");
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
fn export_list_component() {
    // `export { Foo }` where Foo is a local component const (Cgv-P)
    let names = check_names("const Foo = () => <div/>;\nexport { Foo };");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn export_list_with_alias() {
    // `export { Foo as Bar }` — exported name is Bar
    let names = check_names("const Foo = () => <div/>;\nexport { Foo as Bar };");
    assert_eq!(names, vec!["Bar"]);
}

#[test]
fn export_list_non_component_ignored() {
    // `export { foo }` where foo is lowercase — not in local_vars (not a component)
    let names = check_names("const foo = () => <div/>;\nexport { foo };");
    assert!(names.is_empty());
}

#[test]
fn export_class_extends_component() {
    // `export class Foo extends Component {}` (Cgv-R)
    let names = check_names("export class Foo extends Component {}");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn export_class_extends_react_component() {
    // `export class Foo extends React.Component {}`
    let names = check_names("export class Foo extends React.Component {}");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn export_class_no_superclass_ignored() {
    // Plain class without extending Component is not a React component
    let names = check_names("export class Foo {}");
    assert!(names.is_empty());
}

#[test]
fn export_class_extends_pure_component() {
    // `export class Foo extends PureComponent {}`
    let names = check_names("export class Foo extends PureComponent {}");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn local_class_component_resolves_via_export_list() {
    // class Foo extends Component defined locally, exported via { Foo }
    let names = check_names("class Foo extends Component {}\nexport { Foo };");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn export_default_function_decl_resolved() {
    // `function Page() {}; export default Page;` — FunctionDeclaration in first-pass local_vars
    let names = check_names("function Page() { return <div/>; }\nexport default Page;");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_list_resolves_function_decl() {
    // `function Foo() {}; export { Foo };` — FunctionDeclaration via export-list
    let names = check_names("function Foo() { return <div/>; }\nexport { Foo };");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn export_default_class_no_superclass_not_component() {
    // Default class without Component ancestry — should NOT be extracted (P2 guard)
    let names = check_names("export default class Foo {}");
    assert!(names.is_empty());
}

#[test]
fn export_default_class_extends_component_is_component() {
    let names = check_names("export default class Foo extends Component {}");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_memo_wraps_identifier_uses_decl_span() {
    // `const Page = (props) => ...; export default memo(Page)` — span should cover Page's decl
    // We just verify extraction succeeds and the name is "default"
    let names = check_names("const Page = (props) => <div/>;\nexport default memo(Page);");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_default_dynamic_extracted_as_component() {
    // `export default dynamic(...)` should be extracted (dynamic added to callee list)
    let names = check_names("export default dynamic(() => import('./Heavy'));");
    assert_eq!(names, vec!["default"]);
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

#[test]
fn local_var_with_non_component_init_ignored() {
    // `const Foo = 42` — component name but non-component expr init; exercises line 37
    // (closing } of `if let Some(init)` when is_component_expr returns false)
    let names = check_names("const Foo = 42;\nexport default Foo;");
    assert!(names.is_empty());
}

#[test]
fn local_destructured_var_not_tracked() {
    // `const [Foo] = []` — ArrayPattern binding, not BindingIdentifier; exercises line 39
    let names = check_names("const [Foo] = [];\nexport default Foo;");
    assert!(names.is_empty());
}

#[test]
fn export_default_memo_no_args_is_component() {
    // `export default memo()` — no args; exercises the `else { span }` branch at line 104
    let names = check_names("export default memo();");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn export_named_ts_type_ignored() {
    // TypeScript type alias hits the `_ => {}` arm in ExportNamedDeclaration (line 172)
    let names = check_names("export type Foo = {};");
    assert!(names.is_empty());
}

#[test]
fn local_var_no_init_not_tracked() {
    // `let Foo;` — no init exercises the None path of if let Some(init) (line 37)
    let names = check_names("let Foo;\nexport default Foo;");
    assert!(names.is_empty());
}

#[test]
fn export_const_dynamic_wrapped() {
    // `export const Foo = dynamic(...)` — dynamic added to is_component_expr callee list
    let names = check_names("export const Foo = dynamic(() => import('./Heavy'));");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn export_class_extends_call_not_component() {
    // `class Foo extends getBase()` — superclass is a CallExpression; hits `_ => false`
    // in is_class_component (helpers.rs line 13)
    let names = check_names("export default class Foo extends getBase() {}");
    assert!(names.is_empty());
}

#[test]
fn is_component_expr_non_react_static_member_not_component() {
    // Foo.memo(...) — StaticMemberExpression but object is not React; hits `_ => return false`
    assert!(!check_is_expr("Foo.memo(() => <div/>)"));
}

#[test]
fn is_component_expr_react_memo_is_component() {
    // React.memo(...) — StaticMemberExpression with React object; is a component
    assert!(check_is_expr("React.memo(() => <div/>)"));
}

#[test]
fn export_default_non_react_member_call_not_component() {
    // `export default Foo.memo(...)` — non-React static member callee; not a component
    let names = check_names("export default Foo.memo(() => <div/>);");
    assert!(names.is_empty());
}
