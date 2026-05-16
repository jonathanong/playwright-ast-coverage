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
fn detects_next_dynamic_import() {
    assert!(check(
        "import dynamic from 'next/dynamic'; export default function App() { return <div/>; }"
    ));
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
