use super::collect_fetch_calls;
use crate::ast;
use std::path::PathBuf;

fn fixture(category: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(category)
        .join(name)
}

fn check(fixture_path: &std::path::Path) -> usize {
    let source = std::fs::read_to_string(fixture_path).expect("fixture file must be readable");
    let span = oxc_span::Span::new(0, source.len() as u32);
    ast::with_program(fixture_path, &source, |program, _| {
        collect_fetch_calls(
            program,
            &source,
            fixture_path.to_str().unwrap_or("test.tsx"),
            span,
        )
        .len()
    })
    .unwrap()
}

#[test]
fn detects_fetch_call() {
    let path = fixture("react-traits-fetch", "detect-fetch").join("test.tsx");
    assert_eq!(check(&path), 1);
}

#[test]
fn no_fetch() {
    let path = fixture("react-traits-fetch", "no-fetch").join("test.tsx");
    assert_eq!(check(&path), 0);
}
