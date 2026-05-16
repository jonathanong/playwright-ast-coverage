use super::*;
use tempfile::TempDir;

fn write(dir: &Path, rel: &str, content: &str) -> PathBuf {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, content).unwrap();
    path
}

/// Render `args` through the same `collect_entries` pipeline production uses,
/// then serialize via the requested formatter into a String. Keeps tests in
/// lockstep with `run()` — adding new setup logic to `collect_entries`
/// automatically exercises in tests.
fn run_capture(args: SymbolsArgs) -> String {
    let (entries, root_strs) = collect_entries(&args).unwrap();
    let mut buf = Vec::new();
    let format = args.format.unwrap_or(Format::Json);
    match format {
        Format::Json => output::write_json(&root_strs, &entries, &mut buf).unwrap(),
        Format::Md => output::write_md(&root_strs, &entries, &mut buf).unwrap(),
        Format::Yml => output::write_yml(&root_strs, &entries, &mut buf).unwrap(),
        Format::Paths => output::write_paths(&entries, &mut buf).unwrap(),
        Format::Human => output::write_human(&root_strs, &entries, &mut buf).unwrap(),
    }
    String::from_utf8(buf).unwrap()
}

fn args_for(root: &Path, files: Vec<&str>, format: Format) -> SymbolsArgs {
    SymbolsArgs {
        files: files.into_iter().map(PathBuf::from).collect(),
        root: Some(root.to_path_buf()),
        tsconfig: None,
        kinds: vec![],
        include: Include::Exports,
        format: Some(format),
        json: false,
        timings: false,
    }
}

// ── JSON output: shape ───────────────────────────────────────────────────

#[test]
fn json_simple_exports() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "src/utils.mts",
        "export function foo() {}\nexport const x = 1;\n",
    );
    let out = run_capture(args_for(dir.path(), vec!["src/utils.mts"], Format::Json));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["roots"][0], "src/utils.mts");
    let exports = v["files"][0]["exports"].as_array().unwrap();
    assert_eq!(exports.len(), 2);
    assert_eq!(exports[0]["name"], "foo");
    assert_eq!(exports[0]["kind"], "function");
    assert_eq!(exports[1]["name"], "x");
    assert_eq!(exports[1]["kind"], "const");
}

#[test]
fn json_imports_excluded_by_default() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "src/main.mts",
        "import { x } from './x.mts';\nexport function go() {}\n",
    );
    let out = run_capture(args_for(dir.path(), vec!["src/main.mts"], Format::Json));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    // Default --include=exports omits imports entirely.
    assert!(v["files"][0].get("imports").is_none());
    assert_eq!(v["files"][0]["exports"][0]["name"], "go");
}

#[test]
fn json_include_both() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/x.mts", "export function x() {}\n");
    write(
        dir.path(),
        "src/main.mts",
        "import { x } from './x.mts';\nexport function go() { x(); }\n",
    );
    let mut args = args_for(dir.path(), vec!["src/main.mts"], Format::Json);
    args.include = Include::Both;
    let out = run_capture(args);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let imports = v["files"][0]["imports"].as_array().unwrap();
    assert_eq!(imports[0]["imported"], "x");
    assert_eq!(imports[0]["local"], "x");
    assert_eq!(imports[0]["typeOnly"], false);
    assert_eq!(imports[0]["resolved"], "src/x.mts");
}

#[test]
fn json_reexport_resolves_source() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/inner.mts", "export function deep() {}\n");
    write(
        dir.path(),
        "src/index.mts",
        "export { deep } from './inner.mts';\n",
    );
    let out = run_capture(args_for(dir.path(), vec!["src/index.mts"], Format::Json));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let exp = &v["files"][0]["exports"][0];
    assert_eq!(exp["name"], "deep");
    assert_eq!(exp["kind"], "re-export");
    assert_eq!(exp["reExport"]["source"], "./inner.mts");
    assert_eq!(exp["reExport"]["imported"], "deep");
    assert_eq!(exp["reExport"]["resolved"], "src/inner.mts");
}

#[test]
fn json_reexport_unresolvable_omits_resolved() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "src/index.mts",
        "export { thing } from 'some-npm-pkg';\n",
    );
    let out = run_capture(args_for(dir.path(), vec!["src/index.mts"], Format::Json));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let exp = &v["files"][0]["exports"][0];
    assert_eq!(exp["reExport"]["source"], "some-npm-pkg");
    // Unresolved source: `resolved` field should be absent.
    assert!(exp["reExport"].get("resolved").is_none());
}

#[test]
fn json_type_only_import_resolves() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/types.mts", "export type T = number;\n");
    write(
        dir.path(),
        "src/main.mts",
        "import type { T } from './types.mts';\nexport function go(): void {}\n",
    );
    let mut args = args_for(dir.path(), vec!["src/main.mts"], Format::Json);
    args.include = Include::Both;
    let out = run_capture(args);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let imp = &v["files"][0]["imports"][0];
    assert_eq!(imp["imported"], "T");
    assert_eq!(imp["typeOnly"], true);
    assert_eq!(imp["resolved"], "src/types.mts");
}

// ── --kind filter ────────────────────────────────────────────────────────

#[test]
fn kind_filter_only_function() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "src/m.mts",
        "export function f() {}\nexport const c = 1;\nexport type T = string;\n",
    );
    let mut args = args_for(dir.path(), vec!["src/m.mts"], Format::Json);
    args.kinds = vec![ExportKindArg::Function];
    let out = run_capture(args);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let exports = v["files"][0]["exports"].as_array().unwrap();
    assert_eq!(exports.len(), 1);
    assert_eq!(exports[0]["name"], "f");
}

#[test]
fn kind_filter_multiple() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "src/m.mts",
        "export function f() {}\nexport const c = 1;\nexport type T = string;\nexport interface I { x: number }\n",
    );
    let mut args = args_for(dir.path(), vec!["src/m.mts"], Format::Json);
    args.kinds = vec![ExportKindArg::Function, ExportKindArg::Type];
    let out = run_capture(args);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let names: Vec<_> = v["files"][0]["exports"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["f", "T"]);
}

// Note: invalid `--kind` values can no longer be constructed through `SymbolsArgs`
// — clap's ValueEnum rejects them at parse time. The CLI integration test
// `kind_filter_invalid_value_exits_nonzero` covers that boundary.

// ── --include variants ───────────────────────────────────────────────────

#[test]
fn include_imports_only() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/x.mts", "export function x() {}\n");
    write(
        dir.path(),
        "src/m.mts",
        "import { x } from './x.mts';\nexport function go() {}\n",
    );
    let mut args = args_for(dir.path(), vec!["src/m.mts"], Format::Json);
    args.include = Include::Imports;
    let out = run_capture(args);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["files"][0].get("exports").is_none());
    assert_eq!(v["files"][0]["imports"].as_array().unwrap().len(), 1);
}

// ── Output formats ───────────────────────────────────────────────────────

#[test]
fn paths_format_emits_path_line_name() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "src/m.mts",
        "export function alpha() {}\nexport const beta = 2;\n",
    );
    let out = run_capture(args_for(dir.path(), vec!["src/m.mts"], Format::Paths));
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "src/m.mts:1:alpha");
    assert_eq!(lines[1], "src/m.mts:2:beta");
}

#[test]
fn human_format_lists_exports() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/m.mts", "export function go() {}\n");
    let out = run_capture(args_for(dir.path(), vec!["src/m.mts"], Format::Human));
    assert!(out.contains("src/m.mts"));
    assert!(out.contains("export"));
    assert!(out.contains("function"));
    assert!(out.contains("go"));
}

#[test]
fn human_format_reexport_shows_resolved_path() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/inner.mts", "export function deep() {}\n");
    write(
        dir.path(),
        "src/index.mts",
        "export { deep } from './inner.mts';\n",
    );
    let out = run_capture(args_for(dir.path(), vec!["src/index.mts"], Format::Human));
    // The human format should display the resolved path, not the raw './inner.mts' specifier.
    assert!(
        out.contains("src/inner.mts"),
        "expected resolved path in human output, got: {out}"
    );
    assert!(!out.contains("./inner.mts"));
}

#[test]
fn human_format_reexport_falls_back_to_specifier_when_unresolvable() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "src/index.mts",
        "export { thing } from 'some-npm-pkg';\n",
    );
    let out = run_capture(args_for(dir.path(), vec!["src/index.mts"], Format::Human));
    // Bare npm specifier doesn't resolve — fall back to the original.
    assert!(out.contains("some-npm-pkg"));
}

#[test]
fn md_format_emits_headings() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/m.mts", "export function go() {}\n");
    let out = run_capture(args_for(dir.path(), vec!["src/m.mts"], Format::Md));
    assert!(out.contains("# `src/m.mts`"));
    assert!(out.contains("### Exports"));
    assert!(out.contains("`go`"));
}

#[test]
fn yml_format_round_trips() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/m.mts", "export function go() {}\n");
    let out = run_capture(args_for(dir.path(), vec!["src/m.mts"], Format::Yml));
    let v: serde_yaml::Value = serde_yaml::from_str(&out).unwrap();
    assert_eq!(v["roots"][0].as_str().unwrap(), "src/m.mts");
    assert_eq!(v["files"][0]["exports"][0]["name"].as_str().unwrap(), "go");
}

// ── Multiple files ───────────────────────────────────────────────────────

#[test]
fn multiple_files_each_appear_in_output() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/a.mts", "export function a() {}\n");
    write(dir.path(), "src/b.mts", "export function b() {}\n");
    let out = run_capture(args_for(
        dir.path(),
        vec!["src/a.mts", "src/b.mts"],
        Format::Json,
    ));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let paths: Vec<_> = v["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap().to_string())
        .collect();
    assert!(paths.contains(&"src/a.mts".to_string()));
    assert!(paths.contains(&"src/b.mts".to_string()));
}

// ── Empty file ───────────────────────────────────────────────────────────

#[test]
fn empty_file_omits_exports_field() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/empty.mts", "");
    let out = run_capture(args_for(dir.path(), vec!["src/empty.mts"], Format::Json));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    // exports skipped (empty Vec); confirm path is still present.
    assert_eq!(v["files"][0]["path"], "src/empty.mts");
    assert!(v["files"][0].get("exports").is_none());
}

#[test]
fn tsx_file_parses() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "src/Comp.tsx",
        "export function Comp() { return <div/>; }\n",
    );
    let out = run_capture(args_for(dir.path(), vec!["src/Comp.tsx"], Format::Json));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["files"][0]["exports"][0]["name"], "Comp");
}

// ── Output formatter edge cases ──────────────────────────────────────────

#[test]
fn output_md_no_symbols() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/empty.mts", "");
    let out = run_capture(args_for(dir.path(), vec!["src/empty.mts"], Format::Md));
    assert!(out.contains("_No symbols found._"));
}

#[test]
fn output_human_no_symbols() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/empty.mts", "");
    let out = run_capture(args_for(dir.path(), vec!["src/empty.mts"], Format::Human));
    assert!(out.contains("(no symbols)"));
}

#[test]
fn aliased_import_records_local_and_imported() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/x.mts", "export function realName() {}\n");
    write(
        dir.path(),
        "src/m.mts",
        "import { realName as alias } from './x.mts';\nexport function go() { alias(); }\n",
    );
    let mut args = args_for(dir.path(), vec!["src/m.mts"], Format::Json);
    args.include = Include::Both;
    let out = run_capture(args);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let imp = &v["files"][0]["imports"][0];
    assert_eq!(imp["imported"], "realName");
    assert_eq!(imp["local"], "alias");
}

// ── Coverage gap fillers ─────────────────────────────────────────────────

#[test]
fn paths_format_includes_imports_with_local_name() {
    // `paths` mode emits one line per export and per import; for imports the third
    // column is the local binding name (so an aliased import shows the alias).
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/x.mts", "export function realName() {}\n");
    write(
        dir.path(),
        "src/m.mts",
        "import { realName as alias } from './x.mts';\nexport function go() {}\n",
    );
    let mut args = args_for(dir.path(), vec!["src/m.mts"], Format::Paths);
    args.include = Include::Both;
    let out = run_capture(args);
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines.iter().any(|l| l.ends_with(":alias")));
    assert!(lines.iter().any(|l| l.ends_with(":go")));
}

#[test]
fn import_with_unresolvable_source_omits_resolved() {
    // Bare npm specifiers don't resolve through ts_resolver — the JSON should
    // emit the import without a `resolved` field.
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "src/m.mts",
        "import express from 'express';\nexport function go() {}\n",
    );
    let mut args = args_for(dir.path(), vec!["src/m.mts"], Format::Json);
    args.include = Include::Both;
    let out = run_capture(args);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let imp = &v["files"][0]["imports"][0];
    assert_eq!(imp["source"], "express");
    assert_eq!(imp["imported"], "default");
    assert!(imp.get("resolved").is_none());
}

#[test]
fn human_format_imports_show_type_tag_and_alias() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "src/m.mts",
        "import type { T as Aliased } from './t.mts';\nexport function go(): void {}\n",
    );
    let mut args = args_for(dir.path(), vec!["src/m.mts"], Format::Human);
    args.include = Include::Both;
    let out = run_capture(args);
    // Type-only marker appears.
    assert!(out.contains("(type)"), "expected (type) tag, got: {out}");
    // Alias renders as `imported as local`.
    assert!(
        out.contains("T as Aliased"),
        "expected 'T as Aliased' alias rendering, got: {out}"
    );
}

#[test]
fn human_format_multiple_files_lists_each_with_blank_separators() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/a.mts", "export function alpha() {}\n");
    write(dir.path(), "src/b.mts", "export function beta() {}\n");
    let out = run_capture(args_for(
        dir.path(),
        vec!["src/a.mts", "src/b.mts"],
        Format::Human,
    ));
    assert!(out.contains("2 files"));
    assert!(out.contains("src/a.mts"));
    assert!(out.contains("src/b.mts"));
    assert!(out.contains("alpha"));
    assert!(out.contains("beta"));
}

#[test]
fn md_format_multiple_files_emits_per_file_subheadings() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/a.mts", "export function alpha() {}\n");
    write(dir.path(), "src/b.mts", "export function beta() {}\n");
    let out = run_capture(args_for(
        dir.path(),
        vec!["src/a.mts", "src/b.mts"],
        Format::Md,
    ));
    assert!(out.contains("# 2 files"));
    assert!(out.contains("## `src/a.mts`"));
    assert!(out.contains("## `src/b.mts`"));
}

#[test]
fn md_format_imports_render_aliased_and_type_only() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "src/m.mts",
        "import type { T as Aliased } from './t.mts';\nexport function go() {}\n",
    );
    let mut args = args_for(dir.path(), vec!["src/m.mts"], Format::Md);
    args.include = Include::Both;
    let out = run_capture(args);
    assert!(out.contains("### Imports"));
    assert!(
        out.contains("`T` as `Aliased` from `./t.mts`"),
        "expected aliased import line, got: {out}"
    );
    assert!(out.contains("(type-only)"));
}

#[test]
fn export_kind_default_serializes_as_default() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "src/m.mts",
        "export default function handler() {}\n",
    );
    let out = run_capture(args_for(dir.path(), vec!["src/m.mts"], Format::Json));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["files"][0]["exports"][0]["kind"], "default");
}

#[test]
fn md_format_reexport_uses_resolved_path() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/inner.mts", "export function deep() {}\n");
    write(
        dir.path(),
        "src/index.mts",
        "export { deep } from './inner.mts';\n",
    );
    let out = run_capture(args_for(dir.path(), vec!["src/index.mts"], Format::Md));
    // Resolved path takes precedence over the raw './inner.mts' specifier.
    assert!(
        out.contains("from `src/inner.mts`"),
        "expected resolved path in md output, got: {out}"
    );
    assert!(!out.contains("from `./inner.mts`"));
}

#[test]
fn md_format_import_uses_resolved_path() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/util.mts", "export function helper() {}\n");
    write(
        dir.path(),
        "src/main.mts",
        "import { helper } from './util.mts';\nexport function go() {}\n",
    );
    let mut args = args_for(dir.path(), vec!["src/main.mts"], Format::Md);
    args.include = Include::Both;
    let out = run_capture(args);
    assert!(
        out.contains("from `src/util.mts`"),
        "expected resolved path in md import, got: {out}"
    );
    assert!(!out.contains("from `./util.mts`"));
}

#[test]
fn md_format_multiple_files_lists_root_paths_under_heading() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/a.mts", "export function a() {}\n");
    write(dir.path(), "src/b.mts", "export function b() {}\n");
    let out = run_capture(args_for(
        dir.path(),
        vec!["src/a.mts", "src/b.mts"],
        Format::Md,
    ));
    // Multi-root heading is followed by a bulleted list of the input paths
    // (mirrors the dependencies binary's md output).
    assert!(out.contains("# 2 files"));
    assert!(out.contains("- `src/a.mts`"));
    assert!(out.contains("- `src/b.mts`"));
}

#[test]
fn human_format_import_uses_resolved_path() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/util.mts", "export function helper() {}\n");
    write(
        dir.path(),
        "src/main.mts",
        "import { helper } from './util.mts';\nexport function go() {}\n",
    );
    let mut args = args_for(dir.path(), vec!["src/main.mts"], Format::Human);
    args.include = Include::Both;
    let out = run_capture(args);
    assert!(
        out.contains("from src/util.mts"),
        "expected resolved path in human import line, got: {out}"
    );
    assert!(!out.contains("from ./util.mts"));
}

#[test]
fn export_kind_enum_serializes_as_enum() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "src/m.mts",
        "export enum Color { Red, Green, Blue }\n",
    );
    let out = run_capture(args_for(dir.path(), vec!["src/m.mts"], Format::Json));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["files"][0]["exports"][0]["kind"], "enum");
    assert_eq!(v["files"][0]["exports"][0]["name"], "Color");
}
