use super::build_import_table;
use crate::ast;
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/react-traits-analyze/import-table")
}

fn build_table_from_fixture(fixture_name: &str) -> super::ImportTable {
    let path = fixture_dir().join(fixture_name);
    let source = std::fs::read_to_string(&path).expect("fixture must be readable");
    ast::with_program(&path, &source, |program, _| {
        build_import_table(&path, program)
    })
    .unwrap()
}

#[test]
fn default_import_resolved() {
    let table = build_table_from_fixture("default-import.tsx");
    let entry = table.get("Foo").expect("Foo should be in table");
    assert_eq!(entry.exported_name, "default");
    assert!(entry.resolved_path.ends_with("Foo.tsx"));
}

#[test]
fn namespace_import_resolved() {
    let table = build_table_from_fixture("namespace-import.tsx");
    let entry = table.get("Foo").expect("Foo should be in table");
    assert_eq!(entry.exported_name, "*");
}

#[test]
fn named_import_resolved() {
    let table = build_table_from_fixture("named-import.tsx");
    let entry = table.get("Bar").expect("Bar should be in table");
    assert_eq!(entry.exported_name, "Bar");
}

#[test]
fn type_only_import_skipped() {
    let table = build_table_from_fixture("type-only-import.tsx");
    assert!(table.is_empty(), "type import should be skipped");
}

#[test]
fn unresolvable_import_skipped() {
    let table = build_table_from_fixture("unresolvable-import.tsx");
    assert!(table.is_empty(), "unresolvable import should be skipped");
}

#[test]
fn non_relative_import_skipped() {
    let table = build_table_from_fixture("non-relative-import.tsx");
    assert!(table.is_empty(), "non-relative import should be skipped");
}

#[test]
fn side_effect_import_skipped() {
    let table = build_table_from_fixture("side-effect-import.tsx");
    assert!(
        table.is_empty(),
        "side-effect import should produce no table entries"
    );
}
