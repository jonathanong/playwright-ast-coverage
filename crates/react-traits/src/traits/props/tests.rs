use super::detect_props;
use no_mistakes_core::ast;

fn check(source: &str) -> (bool, bool) {
    let path = std::path::Path::new("test.tsx");
    let span = oxc_span::Span::new(0, source.len() as u32);
    ast::with_program(path, source, |program, _| detect_props(program, span)).unwrap()
}

#[test]
fn no_props_no_passes() {
    let (has_props, passes_props) = check("export default function App() { return <div/>; }");
    assert!(!has_props);
    assert!(!passes_props);
}

#[test]
fn has_props_from_params() {
    let (has_props, _) = check("export default function App({ name }) { return <div/>; }");
    assert!(has_props);
}

#[test]
fn passes_props_to_child() {
    let (_, passes_props) = check("export default function App() { return <Child name=\"x\" />; }");
    assert!(passes_props);
}

#[test]
fn passes_props_spread_attribute() {
    let (_, passes_props) =
        check("export default function App(props) { return <Child {...props} />; }");
    assert!(passes_props);
}

#[test]
fn has_props_named_export_function() {
    let (has_props, _) = check("export function Foo({ name }) { return <div/>; }");
    assert!(has_props);
}

#[test]
fn has_props_named_export_arrow() {
    let (has_props, _) = check("export const Foo = (props) => <div/>;");
    assert!(has_props);
}

#[test]
fn no_props_named_export_arrow_no_params() {
    // Arrow with no params — exercises the fall-through path in VariableDeclaration branch.
    let (has_props, _) = check("export const Foo = () => <div/>;");
    assert!(!has_props);
}

#[test]
fn no_props_named_export_class() {
    // Class declaration as named export — hits the _ => {} arm in named export match.
    let (has_props, _) = check("export class Foo {}");
    assert!(!has_props);
}

#[test]
fn no_props_named_export_const_non_arrow() {
    // Variable export where init is not an arrow function — exercises fall-through
    // of the `if let Some(ArrowFunctionExpression)` check.
    let (has_props, _) = check("export const foo = 42;");
    assert!(!has_props);
}

#[test]
fn no_props_named_reexport() {
    // Re-export (no declaration) — e.declaration is None, exercises the outer if-let fall-through.
    let (has_props, _) = check("export { foo } from 'some-module';");
    assert!(!has_props);
}

#[test]
fn has_props_default_arrow() {
    let (has_props, _) = check("export default ({ name }) => <div/>;");
    assert!(has_props);
}

#[test]
fn has_props_default_fn_expression() {
    let (has_props, _) = check("export default function(props) { return <div/>; }");
    assert!(has_props);
}

#[test]
fn passes_props_to_member_expression_component() {
    // <Foo.Bar prop="x"/> — MemberExpression branch returns true for is_component
    let (_, passes_props) =
        check("export default function App() { return <Foo.Bar prop=\"x\" />; }");
    assert!(passes_props);
}

#[test]
fn has_props_named_export_function_expression() {
    // export const Foo = function(props) {} — function expression in var decl (ChwMP)
    let (has_props, _) = check("export const Foo = function(props) { return <div/>; };");
    assert!(has_props);
}

#[test]
fn has_props_memo_wrapped_function() {
    // export default memo(function App(props) {}) — props inside wrapper (Chpev)
    let (has_props, _) = check("export default memo(function App(props) { return <div/>; });");
    assert!(has_props);
}

#[test]
fn has_props_memo_wrapped_arrow() {
    // export default memo((props) => <div/>) — arrow inside memo wrapper
    let (has_props, _) = check("export default memo((props) => <div/>);");
    assert!(has_props);
}

#[test]
fn no_props_memo_wrapped_no_params() {
    // export default memo(() => <div/>) — no params, exercises the CallExpression branch
    let (has_props, _) = check("export default memo(() => <div/>);");
    assert!(!has_props);
}

#[test]
fn has_props_default_identifier_export_with_arrow() {
    // `const Page = (props) => ...; export default Page;` — detects props via local decl span
    let (has_props, _) = check("const Page = (props) => <div/>;\nexport default Page;");
    assert!(has_props);
}

#[test]
fn has_props_default_identifier_export_with_function() {
    // `function Page(props) {}; export default Page;` — detects props via FunctionDeclaration span
    let (has_props, _) = check("function Page(props) { return <div/>; }\nexport default Page;");
    assert!(has_props);
}

#[test]
fn no_props_default_identifier_export_no_params() {
    // `const Page = () => ...; export default Page;` — no params, should be false
    let (has_props, _) = check("const Page = () => <div/>;\nexport default Page;");
    assert!(!has_props);
}

#[test]
fn has_props_export_list_arrow() {
    // `const Foo = (props) => ...; export { Foo };` — detects props via local decl
    let (has_props, _) = check("const Foo = (props) => <div/>;\nexport { Foo };");
    assert!(has_props);
}

#[test]
fn jsx_props_outside_span_not_detected() {
    // Span that covers nothing — visit_jsx_opening_element returns early (line 22-24).
    let source = "export default function App() { return <Child name=\"x\" />; }";
    let path = std::path::Path::new("test.tsx");
    let (_, passes_props) = no_mistakes_core::ast::with_program(path, source, |program, _| {
        super::detect_props(program, oxc_span::Span::new(0, 0))
    })
    .unwrap();
    assert!(
        !passes_props,
        "passes_props should be false when span is empty"
    );
}

#[test]
fn has_props_default_identifier_export_with_function_expr() {
    // `const Page = function(props) {}; export default Page;` — FunctionExpression in a
    // non-exported VariableDeclaration; exercises lines 124-125 in detect_props
    let (has_props, _) =
        check("const Page = function(props) { return <div/>; };\nexport default Page;");
    assert!(has_props);
}

#[test]
fn memo_no_args_not_has_props() {
    // `export default memo()` — no args; call.arguments.first() is None; hits line 75
    let (has_props, _) = check("export default memo();");
    assert!(!has_props);
}

#[test]
fn memo_spread_arg_not_has_props() {
    // `export default memo(...fn)` — spread arg; first_arg.as_expression() is None; hits line 74
    let (has_props, _) = check("export default memo(...fn);");
    assert!(!has_props);
}

#[test]
fn non_overlapping_named_decl_declarator_skipped() {
    // `export const A = 1, B = (props) => ...` — A's span does not overlap B's component
    // span, so the loop hits `continue` at line 90
    let source = "export const A = 1, B = (props) => <div/>;";
    let path = std::path::Path::new("test.tsx");
    let result = no_mistakes_core::ast::with_program(path, source, |program, _| {
        let defs = crate::analyze::components::extract_components(program);
        let b = defs.iter().find(|d| d.name == "B").expect("B");
        super::detect_props(program, b.span)
    })
    .unwrap();
    assert!(result.0, "B has props");
}

#[test]
fn non_overlapping_local_decl_declarator_skipped() {
    // `const A = 1, B = (props) => ...; export default B` — A does not overlap B's span;
    // hits `continue` at line 116
    let source = "const A = 1, B = (props) => <div/>;\nexport default B;";
    let path = std::path::Path::new("test.tsx");
    let result = no_mistakes_core::ast::with_program(path, source, |program, _| {
        let defs = crate::analyze::components::extract_components(program);
        let def = defs.first().expect("default");
        super::detect_props(program, def.span)
    })
    .unwrap();
    assert!(result.0, "B has props");
}

#[test]
fn has_props_named_export_memo_wrapped_arrow() {
    // `export const Foo = memo((props) => ...)` — CallExpression in named export var decl
    let (has_props, _) = check("export const Foo = memo((props) => <div/>);");
    assert!(has_props);
}

#[test]
fn has_props_named_export_memo_wrapped_function() {
    // `export const Foo = memo(function(props) {...})` — function inside wrapper
    let (has_props, _) = check("export const Foo = memo(function(props) { return <div/>; });");
    assert!(has_props);
}

#[test]
fn has_props_local_memo_wrapped_then_default_export() {
    // `const Page = memo((props) => ...); export default Page;` — local var decl wrapper
    let (has_props, _) = check("const Page = memo((props) => <div/>);\nexport default Page;");
    assert!(has_props);
}

#[test]
fn no_props_named_export_memo_no_args() {
    // `export const Foo = memo()` — no args; expr_or_wrapped_has_params returns false
    let (has_props, _) = check("export const Foo = memo();");
    assert!(!has_props);
}

#[test]
fn no_props_named_export_memo_spread_arg() {
    // `export const Foo = memo(...fn)` — spread arg; as_expression() is None
    let (has_props, _) = check("export const Foo = memo(...fn);");
    assert!(!has_props);
}

#[test]
fn no_props_named_export_memo_wrapped_no_params() {
    // `export const Foo = memo(() => ...)` — arrow with no params inside wrapper
    let (has_props, _) = check("export const Foo = memo(() => <div/>);");
    assert!(!has_props);
}
