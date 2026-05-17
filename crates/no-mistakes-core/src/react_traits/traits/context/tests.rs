use super::detect_context_provider;
use crate::ast;
use std::path::PathBuf;

fn fixture_source(name: &str) -> (PathBuf, String) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/react-traits-analyze/context")
        .join(name)
        .join("test.tsx");
    let source = std::fs::read_to_string(&path).expect("fixture must be readable");
    (path, source)
}

fn check(name: &str) -> bool {
    let (path, source) = fixture_source(name);
    let span = oxc_span::Span::new(0, source.len() as u32);
    ast::with_program(&path, &source, |program, _| {
        detect_context_provider(program, span)
    })
    .unwrap()
}

#[test]
fn detects_context_provider() {
    assert!(check("with-provider"));
}

#[test]
fn no_context_provider() {
    assert!(!check("without-provider"));
}

#[test]
fn detects_standalone_provider_tag() {
    assert!(check("standalone-provider"));
}

#[test]
fn provider_outside_span_not_detected() {
    let (path, source) = fixture_source("with-provider");
    let result = ast::with_program(&path, &source, |program, _| {
        detect_context_provider(program, oxc_span::Span::new(0, 0))
    })
    .unwrap();
    assert!(!result);
}
