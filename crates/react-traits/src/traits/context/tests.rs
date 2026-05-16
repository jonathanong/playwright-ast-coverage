use super::detect_context_provider;
use no_mistakes_core::ast;

fn check(source: &str) -> bool {
    let path = std::path::Path::new("test.tsx");
    let span = oxc_span::Span::new(0, source.len() as u32);
    ast::with_program(path, source, |program, _| {
        detect_context_provider(program, span)
    })
    .unwrap()
}

#[test]
fn detects_context_provider() {
    assert!(check(
        "export default function App() { return <MyCtx.Provider value={1}><div/></MyCtx.Provider>; }"
    ));
}

#[test]
fn no_context_provider() {
    assert!(!check("export default function App() { return <div/>; }"));
}

#[test]
fn detects_standalone_provider_tag() {
    assert!(check(
        "export default function App() { return <Provider value={1}><div/></Provider>; }"
    ));
}

#[test]
fn provider_outside_span_not_detected() {
    // Span that covers nothing — visit_jsx_opening_element returns early (line 16-17).
    let source =
        "export default function App() { return <MyCtx.Provider value={1}><div/></MyCtx.Provider>; }";
    let path = std::path::Path::new("test.tsx");
    let result = ast::with_program(path, source, |program, _| {
        detect_context_provider(program, oxc_span::Span::new(0, 0))
    })
    .unwrap();
    assert!(!result);
}
