use super::*;

fn syms(src: &str) -> FileSymbols {
    extract_symbols(src, false).unwrap()
}

fn syms_tsx(src: &str) -> FileSymbols {
    extract_symbols(src, true).unwrap()
}

// ── Imports ──────────────────────────────────────────────────────────────

#[test]
fn named_import() {
    let s = syms("import { foo } from './foo.mts';");
    assert_eq!(s.imports.len(), 1);
    assert_eq!(s.imports[0].imported, "foo");
    assert_eq!(s.imports[0].local, "foo");
    assert_eq!(s.imports[0].source, "./foo.mts");
    assert!(!s.imports[0].is_type_only);
}

#[test]
fn named_import_aliased() {
    let s = syms("import { foo as bar } from './foo.mts';");
    assert_eq!(s.imports[0].imported, "foo");
    assert_eq!(s.imports[0].local, "bar");
}

#[test]
fn type_only_import() {
    let s = syms("import type { Foo } from './types.mts';");
    assert_eq!(s.imports[0].imported, "Foo");
    assert!(s.imports[0].is_type_only);
}

#[test]
fn namespace_import() {
    let s = syms("import * as ns from './utils.mts';");
    assert_eq!(s.imports[0].imported, "*");
    assert_eq!(s.imports[0].local, "ns");
}

#[test]
fn default_import() {
    let s = syms("import app from './app.mts';");
    assert_eq!(s.imports[0].imported, "default");
    assert_eq!(s.imports[0].local, "app");
}

#[test]
fn multiple_named_imports() {
    let s = syms("import { a, b, c } from './mod.mts';");
    assert_eq!(s.imports.len(), 3);
    let names: Vec<_> = s.imports.iter().map(|i| i.imported.as_str()).collect();
    assert!(names.contains(&"a"));
    assert!(names.contains(&"b"));
    assert!(names.contains(&"c"));
}

#[test]
fn side_effect_import_has_no_symbols() {
    let s = syms("import './setup.mts';");
    assert!(s.imports.is_empty());
}

#[test]
fn panicked_parse_returns_error() {
    let err = extract_symbols("export const = ;", false).unwrap_err();
    assert!(format!("{err:#}").contains("failed to parse TypeScript source"));
}

// ── Exports — functions and classes ──────────────────────────────────────

#[test]
fn export_function() {
    let s = syms("export function greet() {}");
    assert_eq!(s.exports.len(), 1);
    assert_eq!(s.exports[0].name, "greet");
    assert_eq!(s.exports[0].kind, ExportKind::Function);
}

#[test]
fn export_class() {
    let s = syms("export class Foo {}");
    assert_eq!(s.exports[0].kind, ExportKind::Class);
    assert_eq!(s.exports[0].name, "Foo");
}

#[test]
fn export_const() {
    let s = syms("export const x = 1;");
    assert_eq!(s.exports[0].kind, ExportKind::Const);
    assert_eq!(s.exports[0].name, "x");
}

#[test]
fn export_let() {
    let s = syms("export let y = 2;");
    assert_eq!(s.exports[0].kind, ExportKind::Let);
}

#[test]
fn export_var() {
    let s = syms("export var z = 3;");
    assert_eq!(s.exports[0].kind, ExportKind::Var);
}

#[test]
fn ignored_export_declaration_has_no_symbol() {
    let s = syms("export namespace Internal { export const value = 1; }");
    assert!(s.exports.is_empty());
}

#[test]
fn unnamed_inline_exports_are_ignored() {
    let mut out = FileSymbols::default();
    push_export_if_named(&mut out, None, ExportKind::Function, 1, false);
    assert!(out.exports.is_empty());
}

// ── Exports — type-level ─────────────────────────────────────────────────

#[test]
fn export_type_alias() {
    let s = syms("export type MyType = string;");
    assert_eq!(s.exports[0].kind, ExportKind::TypeAlias);
    assert_eq!(s.exports[0].name, "MyType");
    assert!(s.exports[0].is_type_only);
}

#[test]
fn export_interface() {
    let s = syms("export interface IFoo { x: number; }");
    assert_eq!(s.exports[0].kind, ExportKind::Interface);
    assert_eq!(s.exports[0].name, "IFoo");
    assert!(s.exports[0].is_type_only);
}

#[test]
fn export_enum() {
    let s = syms("export enum Color { Red, Green }");
    assert_eq!(s.exports[0].kind, ExportKind::Enum);
    assert_eq!(s.exports[0].name, "Color");
    assert!(!s.exports[0].is_type_only);
}

// ── Exports — default ───────────────────────────────────────────────────

#[test]
fn export_default_function() {
    let s = syms("export default function handler() {}");
    assert_eq!(s.exports[0].kind, ExportKind::Default);
    assert_eq!(s.exports[0].name, "handler");
}

#[test]
fn export_default_anonymous() {
    let s = syms("export default 42;");
    assert_eq!(s.exports[0].kind, ExportKind::Default);
    assert_eq!(s.exports[0].name, "default");
}

#[test]
fn export_default_class_interface_and_identifier_names() {
    let s = syms(
        r#"
const value = 1;
export default value;
"#,
    );
    assert_eq!(s.exports[0].name, "value");

    let class_symbols = syms("export default class NamedDefault {}");
    assert_eq!(class_symbols.exports[0].name, "NamedDefault");

    let anonymous_class = syms("export default class {}");
    assert_eq!(anonymous_class.exports[0].name, "default");

    let interface_symbols = syms("export default interface DefaultShape {}");
    assert_eq!(interface_symbols.exports[0].name, "DefaultShape");
    assert!(interface_symbols.exports[0].is_type_only);
}

// ── Exports — re-exports ─────────────────────────────────────────────────

#[test]
fn export_named_reexport() {
    let s = syms("export { foo } from './foo.mts';");
    assert_eq!(s.exports.len(), 1);
    match &s.exports[0].kind {
        ExportKind::ReExport { source, imported } => {
            assert_eq!(source, "./foo.mts");
            assert_eq!(imported, "foo");
        }
        _ => panic!("expected ReExport"),
    }
}

#[test]
fn export_type_reexport_forms_are_type_only() {
    let s = syms(
        r#"
export type { Foo } from './foo.mts';
export { type Bar } from './bar.mts';
export type * from './types.mts';
"#,
    );
    assert_eq!(s.exports.len(), 3);
    assert!(s.exports.iter().all(|export| export.is_type_only));
}

#[test]
fn export_renamed_reexport() {
    let s = syms("export { foo as bar } from './foo.mts';");
    assert_eq!(s.exports[0].name, "bar");
    match &s.exports[0].kind {
        ExportKind::ReExport { imported, .. } => assert_eq!(imported, "foo"),
        _ => panic!("expected ReExport"),
    }
}

#[test]
fn export_star_reexport() {
    let s = syms("export * from './module.mts';");
    assert_eq!(s.exports[0].name, "*");
    match &s.exports[0].kind {
        ExportKind::ReExport { source, imported } => {
            assert_eq!(source, "./module.mts");
            assert_eq!(imported, "*");
        }
        _ => panic!("expected star ReExport"),
    }
}

// ── Multiple and mixed ───────────────────────────────────────────────────

#[test]
fn multiple_exports() {
    let s = syms("export function a() {}\nexport const b = 1;");
    assert_eq!(s.exports.len(), 2);
}

#[test]
fn export_destructuring_and_local_specifiers() {
    let s = syms(
        r#"
const local = 1;
export const { a, b: renamed, ...rest } = source;
export let [first, second = fallback, ...others] = values;
export { local as publicLocal };
"#,
    );
    let names: Vec<_> = s
        .exports
        .iter()
        .map(|export| export.name.as_str())
        .collect();
    for expected in ["a", "renamed", "first", "second", "publicLocal"] {
        assert!(names.contains(&expected), "missing export {expected}");
    }
}

#[test]
fn mixed_imports_and_exports() {
    let src = r#"
import { x } from './x.mts';
export function use() { return x; }
"#;
    let s = syms(src);
    assert_eq!(s.imports.len(), 1);
    assert_eq!(s.exports.len(), 1);
}

#[test]
fn empty_source() {
    let s = syms("");
    assert!(s.exports.is_empty());
    assert!(s.imports.is_empty());
}

#[test]
fn tsx_extracts_correctly() {
    let src = r#"
import React from 'react';
export function Component() { return null; }
"#;
    let s = syms_tsx(src);
    assert_eq!(s.imports.len(), 1);
    assert_eq!(s.imports[0].imported, "default");
    assert_eq!(s.exports[0].name, "Component");
}

// ── Line numbers ─────────────────────────────────────────────────────────

#[test]
fn export_line_number() {
    let src = "// comment\nexport function foo() {}";
    let s = syms(src);
    assert_eq!(s.exports[0].line, 2);
}

#[test]
fn import_line_number() {
    let src = "// line 1\nimport { x } from './x.mts';";
    let s = syms(src);
    assert_eq!(s.imports[0].line, 2);
}
