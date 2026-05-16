use super::detect_uses_suspense;
use no_mistakes_core::ast;

fn check(source: &str) -> bool {
    let path = std::path::Path::new("test.tsx");
    ast::with_program(path, source, |program, _| {
        detect_uses_suspense(program, source)
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
