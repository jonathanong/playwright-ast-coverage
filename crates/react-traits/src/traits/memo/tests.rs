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
fn detects_forwardref_wrapper() {
    assert!(check(
        "export default forwardRef(function App(props, ref) { return <div ref={ref}/>; });"
    ));
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
