use super::collect_fetch_calls;
use no_mistakes_core::ast;

fn check(source: &str) -> usize {
    let path = std::path::Path::new("test.tsx");
    let span = oxc_span::Span::new(0, source.len() as u32);
    ast::with_program(path, source, |program, _| {
        collect_fetch_calls(program, source, "test.tsx", span).len()
    })
    .unwrap()
}

#[test]
fn detects_fetch_call() {
    assert_eq!(
        check("export default async function Page() { const r = await fetch('/api'); }"),
        1
    );
}

#[test]
fn no_fetch() {
    assert_eq!(check("export default function App() { return <div/>; }"), 0);
}
