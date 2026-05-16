use super::detect_uses_memo;
use crate::analyze::components::extract_components;
use no_mistakes_core::ast;

fn check(source: &str) -> bool {
    let path = std::path::Path::new("test.tsx");
    let span = oxc_span::Span::new(0, source.len() as u32);
    ast::with_program(path, source, |program, _| {
        let defs = extract_components(program);
        let def =
            defs.first()
                .cloned()
                .unwrap_or_else(|| crate::analyze::components::ComponentDef {
                    name: "default".to_string(),
                    span: oxc_span::Span::default(),
                });
        detect_uses_memo(program, span, &def)
    })
    .unwrap()
}

#[test]
fn detects_use_memo_hook() {
    assert!(check(
        "export default function App() { const x = useMemo(() => 1, []); return <div/>; }"
    ));
}

#[test]
fn detects_memo_wrapper() {
    assert!(check(
        "export default memo(function App() { return <div/>; });"
    ));
}

#[test]
fn no_memo() {
    assert!(!check("export default function App() { return <div/>; }"));
}

#[test]
fn detects_react_memo_wrapper() {
    assert!(check(
        "export default React.memo(function App() { return <div/>; });"
    ));
}

#[test]
fn detects_use_memo_react_dot() {
    assert!(check(
        "export default function App() { const x = React.useMemo(() => 1, []); return <div/>; }"
    ));
}

#[test]
fn forwardref_wrapper_not_memo() {
    // forwardRef wrapping is not classified as usesMemo (Chpet)
    assert!(!check(
        "export default forwardRef(function App(props, ref) { return <div ref={ref}/>; });"
    ));
}

#[test]
fn non_react_member_memo_not_detected() {
    // Foo.memo(...) must not be treated as React.memo (CgvaA)
    assert!(!check(
        "export default Foo.memo(function App() { return <div/>; });"
    ));
}

#[test]
fn memo_wrapper_only_for_default_component() {
    // Named export must not inherit usesMemo=true from export default memo(...) (Chpeq)
    let source = "export const Foo = () => <div/>;\nexport default memo(function Bar() { return <span/>; });";
    let path = std::path::Path::new("test.tsx");
    let span = oxc_span::Span::new(0, source.len() as u32);
    let result = no_mistakes_core::ast::with_program(path, source, |program, _| {
        let defs = crate::analyze::components::extract_components(program);
        let foo_def = defs.iter().find(|d| d.name == "Foo").cloned().unwrap();
        super::detect_uses_memo(program, span, &foo_def)
    })
    .unwrap();
    assert!(!result);
}

#[test]
fn no_memo_other_hook() {
    // A call like someHook() — name is Some("someHook"), not "useMemo",
    // exercises the Some(name) branch where name != "useMemo".
    assert!(!check(
        "export default function App() { someHook(); return <div/>; }"
    ));
}

#[test]
fn no_memo_non_react_static_member() {
    // Foo.something() — StaticMember but object is not React, hits the None branch.
    assert!(!check(
        "export default function App() { Foo.something(); return <div/>; }"
    ));
}

#[test]
fn no_memo_computed_callee() {
    // obj[key]() — computed callee hits _ => None.
    assert!(!check(
        "export default function App() { obj[key](); return <div/>; }"
    ));
}

#[test]
fn no_memo_wrapped_in_other_call() {
    // export default someWrapper(Fn) — callee is not memo/forwardRef,
    // but IS an Identifier, so hits the matches! check returning false (line 44 area).
    assert!(!check(
        "export default someWrapper(function App() { return <div/>; });"
    ));
}

#[test]
fn no_memo_wrapped_computed_callee() {
    // export default obj[key](Fn) — callee is computed, hits _ => "" in is_wrapped_in_memo.
    assert!(!check(
        "export default obj[key](function App() { return <div/>; });"
    ));
}

#[test]
fn named_export_memo_wrapper_detected() {
    // `export const Foo = memo(() => <div/>)` — named export memo wrapper should be usesMemo
    let source = "export const Foo = memo(() => <div/>);";
    let path = std::path::Path::new("test.tsx");
    let span = oxc_span::Span::new(0, source.len() as u32);
    let result = no_mistakes_core::ast::with_program(path, source, |program, _| {
        let defs = crate::analyze::components::extract_components(program);
        let foo_def = defs.iter().find(|d| d.name == "Foo").cloned().unwrap();
        super::detect_uses_memo(program, span, &foo_def)
    })
    .unwrap();
    assert!(result);
}

#[test]
fn named_export_react_memo_wrapper_detected() {
    // `export const Foo = React.memo(() => <div/>)` — React.memo on named export
    let source = "export const Foo = React.memo(() => <div/>);";
    let path = std::path::Path::new("test.tsx");
    let span = oxc_span::Span::new(0, source.len() as u32);
    let result = no_mistakes_core::ast::with_program(path, source, |program, _| {
        let defs = crate::analyze::components::extract_components(program);
        let foo_def = defs.iter().find(|d| d.name == "Foo").cloned().unwrap();
        super::detect_uses_memo(program, span, &foo_def)
    })
    .unwrap();
    assert!(result);
}

#[test]
fn named_export_non_memo_call_not_detected() {
    // `export const Foo = notMemo(...)` — callee is not "memo"; hits closing } at line 70
    let source = "export const Foo = notMemo(() => <div/>);";
    let path = std::path::Path::new("test.tsx");
    let span = oxc_span::Span::new(0, source.len() as u32);
    let result = no_mistakes_core::ast::with_program(path, source, |program, _| {
        let def = crate::analyze::components::ComponentDef {
            name: "Foo".to_string(),
            span: oxc_span::Span::default(),
        };
        super::detect_uses_memo(program, span, &def)
    })
    .unwrap();
    assert!(!result);
}

#[test]
fn named_export_name_mismatch_not_detected() {
    // `export const Bar = memo(...)` when looking for "Foo" — id.name != def.name, hits line 72
    let source = "export const Bar = memo(() => <div/>);";
    let path = std::path::Path::new("test.tsx");
    let span = oxc_span::Span::new(0, source.len() as u32);
    let result = no_mistakes_core::ast::with_program(path, source, |program, _| {
        let def = crate::analyze::components::ComponentDef {
            name: "Foo".to_string(),
            span: oxc_span::Span::default(),
        };
        super::detect_uses_memo(program, span, &def)
    })
    .unwrap();
    assert!(!result);
}

#[test]
fn named_export_destructured_not_detected() {
    // `export const [Foo] = [memo(...)]` — ArrayPattern hits line 73
    let source = "export const [Foo] = [memo(() => <div/>)];";
    let path = std::path::Path::new("test.tsx");
    let span = oxc_span::Span::new(0, source.len() as u32);
    let result = no_mistakes_core::ast::with_program(path, source, |program, _| {
        let def = crate::analyze::components::ComponentDef {
            name: "Foo".to_string(),
            span: oxc_span::Span::default(),
        };
        super::detect_uses_memo(program, span, &def)
    })
    .unwrap();
    assert!(!result);
}

#[test]
fn use_memo_outside_span_not_detected() {
    // Span that covers nothing — visit_call_expression returns early (line 18).
    let source = "export default function App() { const x = useMemo(() => 1, []); return null; }";
    let path = std::path::Path::new("test.tsx");
    let result = no_mistakes_core::ast::with_program(path, source, |program, _| {
        let defs = crate::analyze::components::extract_components(program);
        let def =
            defs.first()
                .cloned()
                .unwrap_or_else(|| crate::analyze::components::ComponentDef {
                    name: "default".to_string(),
                    span: oxc_span::Span::default(),
                });
        super::detect_uses_memo(program, oxc_span::Span::new(0, 0), &def)
    })
    .unwrap();
    assert!(!result);
}

#[test]
fn local_memo_then_default_export_is_memo() {
    // `const Page = memo(...); export default Page;` — span of Page's declarator is used;
    // Statement::VariableDeclaration branch matches by span
    let source = "const Page = memo(() => <div/>);\nexport default Page;";
    let path = std::path::Path::new("test.tsx");
    let span = oxc_span::Span::new(0, source.len() as u32);
    let result = no_mistakes_core::ast::with_program(path, source, |program, _| {
        let defs = crate::analyze::components::extract_components(program);
        let def = defs.first().cloned().unwrap();
        super::detect_uses_memo(program, span, &def)
    })
    .unwrap();
    assert!(
        result,
        "const Page = memo(...); export default Page; should be usesMemo"
    );
}

#[test]
fn alias_export_memo_detected_by_span() {
    // `const Foo = memo(...); export { Foo as Bar };` — def.name is "Bar" but span covers
    // Foo's declarator; Statement::VariableDeclaration branch matches by span
    let source = "const Foo = memo(() => <div/>);\nexport { Foo as Bar };";
    let path = std::path::Path::new("test.tsx");
    let span = oxc_span::Span::new(0, source.len() as u32);
    let result = no_mistakes_core::ast::with_program(path, source, |program, _| {
        let defs = crate::analyze::components::extract_components(program);
        let bar_def = defs.iter().find(|d| d.name == "Bar").cloned().unwrap();
        super::detect_uses_memo(program, span, &bar_def)
    })
    .unwrap();
    assert!(
        result,
        "re-exported alias of memo-wrapped component should be usesMemo"
    );
}

#[test]
fn local_var_span_mismatch_not_detected() {
    // Two local vars; only the one whose span matches def.span should trigger usesMemo.
    // The other var exercises the span-mismatch path (d.span != def.span).
    let source =
        "const Other = () => <div/>;\nconst Page = memo(() => <span/>);\nexport default Page;";
    let path = std::path::Path::new("test.tsx");
    let span = oxc_span::Span::new(0, source.len() as u32);
    let result = no_mistakes_core::ast::with_program(path, source, |program, _| {
        let defs = crate::analyze::components::extract_components(program);
        let def = defs.first().cloned().unwrap();
        super::detect_uses_memo(program, span, &def)
    })
    .unwrap();
    assert!(result, "Page should be detected as memo-wrapped");
}
