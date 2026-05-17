use super::*;

#[test]
fn extracts_dynamic_imports_and_mock_calls() {
    let source = r#"
vi.mock('./a.mts')
jest.unstable_mockModule(`./b.mts`, () => ({}))
await import('./a.mts')
await import(name)
"#;
    let facts = extract(Path::new("x.test.mts"), source).unwrap();
    assert_eq!(facts.mock_specifiers, vec!["./a.mts", "./b.mts"]);
    assert_eq!(facts.dynamic_imports.len(), 2);
    assert_eq!(
        facts.dynamic_imports[0].specifier.as_deref(),
        Some("./a.mts")
    );
    assert_eq!(facts.dynamic_imports[1].specifier, None);
}

#[test]
fn ignores_non_static_or_non_framework_mock_calls() {
    let source = r#"
foo.mock('./ignored.mts')
vi.mock()
vi.mock(name)
await import(`./${name}.mts`)
"#;
    let facts = extract(Path::new("x.test.mts"), source).unwrap();
    assert!(facts.mock_specifiers.is_empty());
    assert_eq!(facts.dynamic_imports[0].specifier, None);
}

#[test]
fn rejects_unsupported_file_extensions() {
    let Err(err) = extract(Path::new("x.txt"), "") else {
        panic!("unsupported file should fail");
    };
    assert!(err
        .to_string()
        .contains("unsupported JavaScript/TypeScript file"));
}
