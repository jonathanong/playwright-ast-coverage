use super::{detect_file_environment, FileEnvironment};
use no_mistakes_core::ast;
use std::path::PathBuf;

fn fixture_source(name: &str) -> (PathBuf, String) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/react-traits-analyze/environment")
        .join(name)
        .join("test.tsx");
    let source = std::fs::read_to_string(&path).expect("fixture must be readable");
    (path, source)
}

fn check(name: &str) -> FileEnvironment {
    let (path, source) = fixture_source(name);
    ast::with_program(&path, &source, |program, _| {
        detect_file_environment(program)
    })
    .unwrap()
}

#[test]
fn detects_use_client() {
    assert_eq!(check("use-client"), FileEnvironment::Client);
}

#[test]
fn detects_use_server() {
    assert_eq!(check("use-server"), FileEnvironment::Server);
}

#[test]
fn unknown_environment() {
    assert_eq!(check("unknown"), FileEnvironment::Unknown);
}

#[test]
fn use_server_wins_over_use_client() {
    assert_eq!(check("server-wins-over-client"), FileEnvironment::Server);
}
