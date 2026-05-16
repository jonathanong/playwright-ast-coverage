use super::detect_uses_suspense;
use no_mistakes_core::ast;

fn check(source: &str) -> bool {
    let path = std::path::Path::new("test.tsx");
    let span = oxc_span::Span::new(0, source.len() as u32);
    ast::with_program(path, source, |program, _| {
        detect_uses_suspense(program, span)
    })
    .unwrap()
}

#[test]
fn detects_suspense_jsx() {
    assert!(check(
        "export default function App() { return <Suspense fallback={null}><div/></Suspense>; }"
    ));
}

#[test]
fn detects_next_dynamic_component_rendered() {
    // dynamic() creates a lazily-loaded component; rendering it in JSX triggers uses_suspense
    assert!(check(
        "const Lazy = dynamic(() => import('./Foo')); export default function App() { return <Lazy/>; }"
    ));
}

#[test]
fn dynamic_import_without_render_not_suspense() {
    // importing next/dynamic without rendering the resulting component = no suspense (Chper)
    assert!(!check(
        "import dynamic from 'next/dynamic'; export default function App() { return <div/>; }"
    ));
}

#[test]
fn detects_react_lazy_component_rendered() {
    // React.lazy() component rendered in JSX within span triggers uses_suspense
    assert!(check(
        "const Lazy = React.lazy(() => import('./Foo')); export default function App() { return <Lazy/>; }"
    ));
}

#[test]
fn dynamic_component_outside_span_not_detected() {
    // dynamic-named JSX element outside span should not trigger suspense
    let source = "const Lazy = dynamic(() => import('./Foo')); export default function App() { return <Lazy/>; }";
    let path = std::path::Path::new("test.tsx");
    let result = no_mistakes_core::ast::with_program(path, source, |program, _| {
        super::detect_uses_suspense(program, oxc_span::Span::new(0, 0))
    })
    .unwrap();
    assert!(!result);
}

#[test]
fn no_suspense() {
    assert!(!check("export default function App() { return <div/>; }"));
}

#[test]
fn detects_react_suspense_member() {
    assert!(check(
        "export default function App() { return <React.Suspense fallback={null}><div/></React.Suspense>; }"
    ));
}

#[test]
fn export_default_dynamic_is_suspense() {
    // `export default dynamic(...)` — component itself is a dynamic wrapper
    assert!(check("export default dynamic(() => import('./Heavy'));"));
}

#[test]
fn export_default_lazy_is_suspense() {
    // `export default lazy(...)` — component itself is a lazy wrapper
    assert!(check("export default lazy(() => import('./Heavy'));"));
}

#[test]
fn export_const_dynamic_component_is_suspense() {
    // `export const Lazy = dynamic(...)` — named export dynamic wrapper is suspense
    assert!(check(
        "export const Lazy = dynamic(() => import('./Heavy'));"
    ));
}

#[test]
fn named_dynamic_component_rendered_from_named_export() {
    // `export const Lazy = dynamic(...)` then render `<Lazy/>` — suspense from rendering it
    assert!(check(
        "export const Lazy = dynamic(() => import('./Heavy')); export default function App() { return <Lazy/>; }"
    ));
}

#[test]
fn suspense_outside_span_not_detected() {
    // Span that covers nothing — visit_jsx_opening_element returns early (line 16-17).
    let source =
        "export default function App() { return <Suspense fallback={null}><div/></Suspense>; }";
    let path = std::path::Path::new("test.tsx");
    let result = no_mistakes_core::ast::with_program(path, source, |program, _| {
        super::detect_uses_suspense(program, oxc_span::Span::new(0, 0))
    })
    .unwrap();
    assert!(!result);
}

#[test]
fn export_default_react_lazy_is_suspense() {
    // `export default React.lazy(...)` — hits StaticMemberExpression arm in is_dynamic_or_lazy_call_by_callee
    assert!(check("export default React.lazy(() => import('./Heavy'));"));
}

#[test]
fn export_default_computed_callee_not_suspense() {
    // computed callee (obj[key]()) — hits `_ => return false` in is_dynamic_or_lazy_call_by_callee
    assert!(!check("export default obj[key]();"));
}

#[test]
fn export_named_non_dynamic_not_suspense() {
    // `export const Foo = notDynamic()` — callee is not dynamic/lazy; exercises false-path
    // closing braces in ExportNamedDeclaration branch of is_component_direct_lazy
    assert!(!check("export const Foo = notDynamic();"));
}

#[test]
fn export_named_no_init_not_suspense() {
    // `export let Foo;` — init is None, exercises the None path of if let Some(init) (line 77)
    assert!(!check("export let Foo;"));
}

#[test]
fn const_memo_not_suspense() {
    // `const Foo = memo(...)` — is_dynamic_or_lazy_call(memo()) is false (memo not in list)
    assert!(!check("const Foo = memo(() => <div/>);"));
}

#[test]
fn const_arrow_init_not_suspense() {
    // ArrowFunctionExpression init — not a CallExpression, hits `return false` in is_dynamic_or_lazy_call
    assert!(!check("const Lazy = () => <div/>;"));
}

#[test]
fn const_computed_callee_not_suspense() {
    // `const Lazy = obj[key]()` — computed callee hits `_ => return false` in is_dynamic_or_lazy_call
    assert!(!check("const Lazy = obj[key]();"));
}

#[test]
fn destructured_var_dynamic_not_detected() {
    // `const [a] = [dynamic(...)]` — ArrayPattern binding hits `continue` in collect_from_var_decl
    assert!(!check("const [a] = [dynamic(() => import('./Foo'))];"));
}

#[test]
fn no_init_var_not_dynamic() {
    // `let Lazy;` — no init hits `continue` in collect_from_var_decl
    assert!(!check("let Lazy;"));
}

#[test]
fn exported_const_dynamic_detected_via_named_branch() {
    // `export const Lazy = dynamic(...)` is outside the passed span, so is_component_direct_lazy
    // returns false; collect_dynamic_names must walk the ExportNamedDeclaration branch to find
    // Lazy, then the JSX visitor detects `<Lazy/>` inside the span.
    let source = "export const Lazy = dynamic(() => import('./Foo')); export default function App() { return <Lazy/>; }";
    let path = std::path::Path::new("test.tsx");
    let app_start = source.find("export default").unwrap() as u32;
    let span = oxc_span::Span::new(app_start, source.len() as u32);
    let result = no_mistakes_core::ast::with_program(path, source, |program, _| {
        super::detect_uses_suspense(program, span)
    })
    .unwrap();
    assert!(
        result,
        "exported dynamic rendered inside span should be suspense"
    );
}

#[test]
fn dynamic_inside_function_body_detected() {
    // `const Lazy = dynamic(...)` inside a function body; DynamicNameCollector visits it
    assert!(check(
        "export default function App() { const Lazy = dynamic(() => import('./Foo')); return <Lazy/>; }"
    ));
}

#[test]
fn lazy_inside_arrow_body_detected() {
    // `const Lazy = React.lazy(...)` inside an arrow function body
    assert!(check(
        "export default () => { const Lazy = React.lazy(() => import('./Foo')); return <Lazy/>; };"
    ));
}

#[test]
fn outer_dynamic_shadowed_by_inner_non_dynamic_not_suspense() {
    // `const Lazy = dynamic(...)` at top-level, but inside App `const Lazy = 1` shadows it.
    // The inner non-dynamic binding should prevent <Lazy/> from triggering usesSuspense.
    let source =
        "const Lazy = dynamic(() => import('./Foo'));\nexport default function App() { const Lazy = 1; return <Lazy/>; }";
    let path = std::path::Path::new("test.tsx");
    let app_start = source.find("export default").unwrap() as u32;
    let span = oxc_span::Span::new(app_start, source.len() as u32);
    let result = no_mistakes_core::ast::with_program(path, source, |program, _| {
        super::detect_uses_suspense(program, span)
    })
    .unwrap();
    assert!(!result, "inner non-dynamic shadow should suppress outer dynamic");
}
