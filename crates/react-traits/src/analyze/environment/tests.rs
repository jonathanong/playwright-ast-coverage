use super::{detect_file_environment, FileEnvironment};
use no_mistakes_core::ast;

fn check(source: &str) -> FileEnvironment {
    let path = std::path::Path::new("test.tsx");
    ast::with_program(path, source, |program, _| detect_file_environment(program)).unwrap()
}

#[test]
fn detects_use_client() {
    assert_eq!(
        check("'use client';\nexport default function App() {}"),
        FileEnvironment::Client
    );
}

#[test]
fn detects_use_server() {
    assert_eq!(
        check("'use server';\nexport default function Action() {}"),
        FileEnvironment::Server
    );
}

#[test]
fn unknown_environment() {
    assert_eq!(
        check("export default function App() {}"),
        FileEnvironment::Unknown
    );
}

#[test]
fn use_server_wins_over_use_client() {
    assert_eq!(
        check("'use server';\n'use client';\nexport default function App() {}"),
        FileEnvironment::Server
    );
}
