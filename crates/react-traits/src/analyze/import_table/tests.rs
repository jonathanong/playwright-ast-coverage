use super::build_import_table;
use no_mistakes_core::ast;
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/react-traits-analyze/import-table")
}

fn build_table_from_source(source: &str, file: &std::path::Path) -> super::ImportTable {
    ast::with_program(file, source, |program, _| build_import_table(file, program)).unwrap()
}

#[test]
fn default_import_resolved() {
    let dir = fixture_dir();
    let consumer = dir.join("Consumer.tsx");
    let source = "import Foo from './Foo';";
    let table = build_table_from_source(source, &consumer);
    let entry = table.get("Foo").expect("Foo should be in table");
    assert_eq!(entry.exported_name, "default");
    assert!(entry.resolved_path.ends_with("Foo.tsx"));
}

#[test]
fn namespace_import_resolved() {
    let dir = fixture_dir();
    let consumer = dir.join("Consumer.tsx");
    let source = "import * as Foo from './Foo';";
    let table = build_table_from_source(source, &consumer);
    let entry = table.get("Foo").expect("Foo should be in table");
    assert_eq!(entry.exported_name, "*");
}

#[test]
fn named_import_resolved() {
    let dir = fixture_dir();
    let consumer = dir.join("Consumer.tsx");
    let source = "import { Bar } from './NamedExport';";
    let table = build_table_from_source(source, &consumer);
    let entry = table.get("Bar").expect("Bar should be in table");
    assert_eq!(entry.exported_name, "Bar");
}

#[test]
fn type_only_import_skipped() {
    let dir = fixture_dir();
    let consumer = dir.join("Consumer.tsx");
    let source = "import type Foo from './Foo';";
    let table = build_table_from_source(source, &consumer);
    assert!(table.is_empty(), "type import should be skipped");
}

#[test]
fn unresolvable_import_skipped() {
    let dir = fixture_dir();
    let consumer = dir.join("Consumer.tsx");
    let source = "import Baz from './does-not-exist';";
    let table = build_table_from_source(source, &consumer);
    assert!(table.is_empty(), "unresolvable import should be skipped");
}

#[test]
fn non_relative_import_skipped() {
    let dir = fixture_dir();
    let consumer = dir.join("Consumer.tsx");
    let source = "import React from 'react';";
    let table = build_table_from_source(source, &consumer);
    assert!(table.is_empty(), "non-relative import should be skipped");
}

#[test]
fn side_effect_import_skipped() {
    // `import './Foo'` has no specifiers — hits the `continue` branch in build_import_table.
    let dir = fixture_dir();
    let consumer = dir.join("Consumer.tsx");
    let source = "import './Foo';";
    let table = build_table_from_source(source, &consumer);
    assert!(
        table.is_empty(),
        "side-effect import should produce no table entries"
    );
}
