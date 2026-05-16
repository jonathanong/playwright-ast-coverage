use super::detect_context_provider;
use no_mistakes_core::ast;

fn check(source: &str) -> bool {
    let path = std::path::Path::new("test.tsx");
    ast::with_program(path, source, |program, _| {
        detect_context_provider(program, source)
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
