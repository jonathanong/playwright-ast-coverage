use super::detect_uses_memo;
use crate::analyze::components::extract_components;
use no_mistakes_core::ast;

fn check(source: &str) -> bool {
    let path = std::path::Path::new("test.tsx");
    ast::with_program(path, source, |program, _| {
        let defs = extract_components(program);
        let def =
            defs.first()
                .cloned()
                .unwrap_or_else(|| crate::analyze::components::ComponentDef {
                    name: "default".to_string(),
                    span: oxc_span::Span::default(),
                });
        detect_uses_memo(program, source, &def)
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
