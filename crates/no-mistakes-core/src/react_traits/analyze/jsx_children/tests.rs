use super::collect_jsx_children;
use crate::react_traits::analyze::import_table::build_import_table;
use crate::ast;
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/react-traits-analyze/import-table")
}

fn load_fixture(name: &str) -> (PathBuf, String) {
    let path = fixture_dir().join(name);
    let source = std::fs::read_to_string(&path).expect("fixture must be readable");
    (path, source)
}

fn collect_children_names_from_fixture(fixture_name: &str) -> Vec<String> {
    let (path, source) = load_fixture(fixture_name);
    ast::with_program(&path, &source, |program, _| {
        let table = build_import_table(&path, program);
        let span = oxc_span::Span::new(0, source.len() as u32);
        collect_jsx_children(program, &table, &path, span)
            .into_iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>()
    })
    .unwrap()
}

#[test]
fn capitalized_tag_resolved_to_import() {
    let names = collect_children_names_from_fixture("jsx-capitalized-tag.tsx");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn member_expression_tag_resolved_to_import() {
    let names = collect_children_names_from_fixture("jsx-member-expression-tag.tsx");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn lowercase_tag_ignored() {
    let names = collect_children_names_from_fixture("jsx-lowercase-tag.tsx");
    assert!(names.is_empty());
}

#[test]
fn tag_not_in_import_table_ignored() {
    let names = collect_children_names_from_fixture("jsx-tag-not-in-table.tsx");
    assert!(names.is_empty());
}

#[test]
fn nested_member_expression_tag_resolved() {
    let names = collect_children_names_from_fixture("jsx-nested-member-expression.tsx");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn this_expression_member_tag_ignored() {
    let names = collect_children_names_from_fixture("jsx-this-expression-member.tsx");
    assert!(names.is_empty());
}

#[test]
fn children_outside_span_excluded() {
    let (path, source) = load_fixture("jsx-children-outside-span.tsx");
    let names = ast::with_program(&path, &source, |program, _| {
        let table = build_import_table(&path, program);
        collect_jsx_children(program, &table, &path, oxc_span::Span::new(0, 0))
            .into_iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>()
    })
    .unwrap();
    assert!(names.is_empty(), "elements outside span should be excluded");
}

#[test]
fn same_file_component_resolved() {
    let (path, source) = load_fixture("jsx-same-file-component.tsx");
    let names = ast::with_program(&path, &source, |program, _| {
        let table = build_import_table(&path, program);
        let span = oxc_span::Span::new(0, source.len() as u32);
        collect_jsx_children(program, &table, &path, span)
            .into_iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>()
    })
    .unwrap();
    assert_eq!(names, vec!["Child"]);
}

#[test]
fn same_file_aliased_export_resolved_by_local_name() {
    let (path, source) = load_fixture("jsx-same-file-aliased-export.tsx");
    let names = ast::with_program(&path, &source, |program, _| {
        let table = build_import_table(&path, program);
        let span = oxc_span::Span::new(0, source.len() as u32);
        collect_jsx_children(program, &table, &path, span)
            .into_iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>()
    })
    .unwrap();
    assert_eq!(names, vec!["Bar"]);
}

#[test]
fn same_file_default_export_identifier_not_mapped_to_other_components() {
    let (path, source) = load_fixture("jsx-same-file-default-export-identifier.tsx");
    let names = ast::with_program(&path, &source, |program, _| {
        let table = build_import_table(&path, program);
        let span = oxc_span::Span::new(0, source.len() as u32);
        collect_jsx_children(program, &table, &path, span)
            .into_iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>()
    })
    .unwrap();
    assert!(
        names.is_empty(),
        "Foo should not be mapped as a child since it is not exported"
    );
}

#[test]
fn namespace_import_member_resolved_to_suffix() {
    let names = collect_children_names_from_fixture("jsx-namespace-import-member.tsx");
    assert_eq!(names, vec!["Bar"]);
}

#[test]
fn destructured_export_var_not_in_local_components() {
    let names = collect_children_names_from_fixture("jsx-destructured-export-var.tsx");
    assert!(names.is_empty());
}

#[test]
fn class_export_not_in_local_components() {
    let names = collect_children_names_from_fixture("jsx-class-export-no-superclass.tsx");
    assert!(names.is_empty());
}

#[test]
fn class_export_extends_component_in_local_components() {
    let names = collect_children_names_from_fixture("jsx-class-export-extends-component.tsx");
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn default_export_memo_wrapping_maps_identifier() {
    let names = collect_children_names_from_fixture("jsx-default-export-memo-wrapping.tsx");
    assert_eq!(names, vec!["default"]);
}

#[test]
fn default_export_call_no_args_no_mapping() {
    let names = collect_children_names_from_fixture("jsx-default-export-call-no-args.tsx");
    assert!(names.is_empty());
}
