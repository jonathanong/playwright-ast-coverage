use crate::analyze::import_shape::{is_runtime_export, is_runtime_import};
use crate::analyze::imports::{collect_identifier_references, is_import_used};
use oxc_ast::ast::Statement;
use std::collections::HashSet;

#[test]
fn test_is_runtime_export_variants() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "
            const constant = 1;
            export type { Foo } from './foo';
            export {};
            export { type Bar } from './bar';
            export { Baz } from './baz';
        ";
    let source_type = oxc_span::SourceType::from_path(std::path::Path::new("test.ts")).unwrap();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    assert!(
        parsed.errors.is_empty(),
        "export parser errors: {:?}",
        parsed.errors
    );
    let exports = parsed
        .program
        .body
        .iter()
        .filter_map(|stmt| match stmt {
            Statement::ExportNamedDeclaration(export) => Some(export),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(exports.len(), 4);
    assert!(!is_runtime_export(exports[0]));
    assert!(is_runtime_export(exports[1]));
    assert!(!is_runtime_export(exports[2]));
    assert!(is_runtime_export(exports[3]));
}

#[test]
fn test_is_runtime_export_declaration_without_named_specifiers_is_runtime() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "const marker = 1;\nexport const foo = 1;";
    let source_type = oxc_span::SourceType::from_path(std::path::Path::new("test.ts")).unwrap();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let export = parsed
        .program
        .body
        .iter()
        .find_map(|statement| {
            if let Statement::ExportNamedDeclaration(export) = statement {
                Some(export)
            } else {
                None
            }
        })
        .expect("expected export declaration");
    assert!(is_runtime_export(export));
}

#[test]
fn test_is_runtime_export_declaration_variants() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "
        export interface Foo { bar: string }
        export type Alias = string;
        export class Bar { method() {} }
        export function baz() {}
        export const qux = 1;
    ";
    let source_type = oxc_span::SourceType::from_path(std::path::Path::new("test.ts")).unwrap();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    assert!(
        parsed.errors.is_empty(),
        "parse errors: {:?}",
        parsed.errors
    );
    let exports: Vec<_> = parsed
        .program
        .body
        .iter()
        .filter_map(|stmt| match stmt {
            Statement::ExportNamedDeclaration(e) => Some(e),
            _ => None,
        })
        .collect();
    assert_eq!(exports.len(), 5);
    assert!(!is_runtime_export(exports[0])); // export interface
    assert!(!is_runtime_export(exports[1])); // export type alias
    assert!(is_runtime_export(exports[2]));  // export class
    assert!(is_runtime_export(exports[3]));  // export function
    assert!(is_runtime_export(exports[4]));  // export const
}

#[test]
fn test_is_import_used_respects_identifier_set() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "const marker = 1;\nimport { used, unused } from './dep';\nused();\n";
    let source_type = oxc_span::SourceType::from_path(std::path::Path::new("test.ts")).unwrap();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let import = parsed
        .program
        .body
        .iter()
        .find_map(|stmt| {
            if let Statement::ImportDeclaration(import) = stmt {
                Some(import)
            } else {
                None
            }
        })
        .expect("expected import declaration");
    let referenced_identifiers = collect_identifier_references(&parsed.program);
    assert!(is_import_used(import, &referenced_identifiers));

    let unused_references: HashSet<String> =
        HashSet::from([String::from("other"), String::from("other2")]);
    assert!(!is_import_used(import, &unused_references));
}

#[test]
fn test_is_import_used_is_false_when_named_import_is_not_referenced() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "const marker = 1;\nimport { used, unused } from './dep';\n";
    let source_type = oxc_span::SourceType::from_path(std::path::Path::new("test.ts")).unwrap();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let import = parsed
        .program
        .body
        .iter()
        .find_map(|stmt| {
            if let Statement::ImportDeclaration(import) = stmt {
                Some(import)
            } else {
                None
            }
        })
        .expect("expected import declaration");
    let referenced_identifiers = HashSet::from([String::from("other"), String::from("marker")]);
    assert!(!is_import_used(import, &referenced_identifiers));
}

#[test]
fn test_is_import_used_with_empty_specifiers_is_included() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "const marker = 1;\nimport {} from './dep';\n";
    let source_type = oxc_span::SourceType::from_path(std::path::Path::new("test.ts")).unwrap();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let import = parsed
        .program
        .body
        .iter()
        .find_map(|stmt| {
            if let Statement::ImportDeclaration(import) = stmt {
                Some(import)
            } else {
                None
            }
        })
        .expect("expected import declaration");
    let referenced_identifiers = collect_identifier_references(&parsed.program);
    assert!(is_import_used(import, &referenced_identifiers));
    assert!(is_import_used(import, &HashSet::new()));
}

#[test]
fn test_is_runtime_import_no_specifiers() {
    // A side-effect import with no specifiers is runtime
    let allocator = oxc_allocator::Allocator::default();
    let source = "import './side-effect';";
    let source_type = oxc_span::SourceType::from_path(std::path::Path::new("test.ts")).unwrap();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let import = parsed
        .program
        .body
        .iter()
        .find_map(|stmt| {
            if let Statement::ImportDeclaration(import) = stmt {
                Some(import)
            } else {
                None
            }
        })
        .expect("expected import declaration");
    assert!(is_runtime_import(import));
}
