use super::*;

fn fixture_root() -> PathBuf {
    crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/symbols-output"),
    )
}

/// Render `args` through the same `collect_entries` pipeline production uses,
/// then serialize via the requested formatter into a String.
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

fn fixture_args(files: Vec<&str>, format: Format) -> SymbolsArgs {
    args_for(&fixture_root(), files, format)
}

#[test]
fn json_simple_exports() {
    let out = run_capture(fixture_args(vec!["src/utils.mts"], Format::Json));
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
    let out = run_capture(fixture_args(vec!["src/main.mts"], Format::Json));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["files"][0].get("imports").is_none());
    assert_eq!(v["files"][0]["exports"][0]["name"], "go");
}

#[test]
fn json_include_both() {
    let mut args = fixture_args(vec!["src/main.mts"], Format::Json);
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
    let out = run_capture(fixture_args(vec!["src/index.mts"], Format::Json));
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
    let out = run_capture(fixture_args(
        vec!["src/unresolved-reexport.mts"],
        Format::Json,
    ));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let exp = &v["files"][0]["exports"][0];
    assert_eq!(exp["reExport"]["source"], "some-npm-pkg");
    assert!(exp["reExport"].get("resolved").is_none());
}

#[test]
fn json_type_only_import_resolves() {
    let mut args = fixture_args(vec!["src/type-main.mts"], Format::Json);
    args.include = Include::Both;
    let out = run_capture(args);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let imp = &v["files"][0]["imports"][0];
    assert_eq!(imp["imported"], "T");
    assert_eq!(imp["typeOnly"], true);
    assert_eq!(imp["resolved"], "src/types.mts");
}

#[test]
fn kind_filter_only_function() {
    let mut args = fixture_args(vec!["src/kinds.mts"], Format::Json);
    args.kinds = vec![ExportKindArg::Function];
    let out = run_capture(args);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let exports = v["files"][0]["exports"].as_array().unwrap();
    assert_eq!(exports.len(), 1);
    assert_eq!(exports[0]["name"], "f");
}

#[test]
fn kind_filter_multiple() {
    let mut args = fixture_args(vec!["src/kinds.mts"], Format::Json);
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

#[test]
fn include_imports_only() {
    let mut args = fixture_args(vec!["src/imports-only.mts"], Format::Json);
    args.include = Include::Imports;
    let out = run_capture(args);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["files"][0].get("exports").is_none());
    assert_eq!(v["files"][0]["imports"].as_array().unwrap().len(), 1);
}

#[test]
fn paths_format_emits_path_line_name() {
    let out = run_capture(fixture_args(vec!["src/m.mts"], Format::Paths));
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "src/m.mts:1:alpha");
    assert_eq!(lines[1], "src/m.mts:2:beta");
}

#[test]
fn human_format_lists_exports() {
    let out = run_capture(fixture_args(vec!["src/human.mts"], Format::Human));
    assert!(out.contains("src/human.mts"));
    assert!(out.contains("export"));
    assert!(out.contains("function"));
    assert!(out.contains("go"));
}

#[test]
fn human_format_reexport_shows_resolved_path() {
    let out = run_capture(fixture_args(vec!["src/index.mts"], Format::Human));
    assert!(out.contains("src/inner.mts"));
    assert!(!out.contains("./inner.mts"));
}

#[test]
fn human_format_reexport_falls_back_to_specifier_when_unresolvable() {
    let out = run_capture(fixture_args(
        vec!["src/unresolved-reexport.mts"],
        Format::Human,
    ));
    assert!(out.contains("some-npm-pkg"));
}

#[test]
fn md_format_emits_headings() {
    let out = run_capture(fixture_args(vec!["src/human.mts"], Format::Md));
    assert!(out.contains("# `src/human.mts`"));
    assert!(out.contains("### Exports"));
    assert!(out.contains("`go`"));
}

#[test]
fn yml_format_round_trips() {
    let out = run_capture(fixture_args(vec!["src/human.mts"], Format::Yml));
    let v: serde_yaml::Value = serde_yaml::from_str(&out).unwrap();
    assert_eq!(v["roots"][0].as_str().unwrap(), "src/human.mts");
    assert_eq!(v["files"][0]["exports"][0]["name"].as_str().unwrap(), "go");
}

#[test]
fn multiple_files_each_appear_in_output() {
    let out = run_capture(fixture_args(vec!["src/a.mts", "src/b.mts"], Format::Json));
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

#[test]
fn empty_file_omits_exports_field() {
    let out = run_capture(fixture_args(vec!["src/empty.mts"], Format::Json));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["files"][0]["path"], "src/empty.mts");
    assert!(v["files"][0].get("exports").is_none());
}

#[test]
fn tsx_file_parses() {
    let out = run_capture(fixture_args(vec!["src/Comp.tsx"], Format::Json));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["files"][0]["exports"][0]["name"], "Comp");
}

#[test]
fn output_md_no_symbols() {
    let out = run_capture(fixture_args(vec!["src/empty.mts"], Format::Md));
    assert!(out.contains("_No symbols found._"));
}

#[test]
fn output_human_no_symbols() {
    let out = run_capture(fixture_args(vec!["src/empty.mts"], Format::Human));
    assert!(out.contains("(no symbols)"));
}

#[test]
fn aliased_import_records_local_and_imported() {
    let mut args = fixture_args(vec!["src/alias-import.mts"], Format::Json);
    args.include = Include::Both;
    let out = run_capture(args);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let imp = &v["files"][0]["imports"][0];
    assert_eq!(imp["imported"], "realName");
    assert_eq!(imp["local"], "alias");
}

#[test]
fn paths_format_includes_imports_with_local_name() {
    let mut args = fixture_args(vec!["src/alias-import.mts"], Format::Paths);
    args.include = Include::Both;
    let out = run_capture(args);
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines.iter().any(|l| l.ends_with(":alias")));
    assert!(lines.iter().any(|l| l.ends_with(":go")));
}

#[test]
fn import_with_unresolvable_source_omits_resolved() {
    let mut args = fixture_args(vec!["src/unresolved-import.mts"], Format::Json);
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
    let mut args = fixture_args(vec!["src/type-alias-import.mts"], Format::Human);
    args.include = Include::Both;
    let out = run_capture(args);
    assert!(out.contains("(type)"), "expected (type) tag, got: {out}");
    assert!(
        out.contains("T as Aliased"),
        "expected 'T as Aliased' alias rendering, got: {out}"
    );
}

#[test]
fn human_format_multiple_files_lists_each_with_blank_separators() {
    let out = run_capture(fixture_args(vec!["src/a.mts", "src/b.mts"], Format::Human));
    assert!(out.contains("2 files"));
    assert!(out.contains("src/a.mts"));
    assert!(out.contains("src/b.mts"));
    assert!(out.contains("alpha"));
    assert!(out.contains("beta"));
}

#[test]
fn md_format_multiple_files_emits_per_file_subheadings() {
    let out = run_capture(fixture_args(vec!["src/a.mts", "src/b.mts"], Format::Md));
    assert!(out.contains("# 2 files"));
    assert!(out.contains("## `src/a.mts`"));
    assert!(out.contains("## `src/b.mts`"));
}

#[test]
fn md_format_imports_render_aliased_and_type_only() {
    let mut args = fixture_args(vec!["src/type-alias-import.mts"], Format::Md);
    args.include = Include::Both;
    let out = run_capture(args);
    assert!(out.contains("### Imports"));
    assert!(out.contains("`T` as `Aliased` from `src/types.mts`"));
    assert!(out.contains("(type-only)"));
}

#[test]
fn export_kind_default_serializes_as_default() {
    let out = run_capture(fixture_args(vec!["src/default.mts"], Format::Json));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["files"][0]["exports"][0]["kind"], "default");
}

#[test]
fn md_format_reexport_uses_resolved_path() {
    let out = run_capture(fixture_args(vec!["src/index.mts"], Format::Md));
    assert!(out.contains("from `src/inner.mts`"));
    assert!(!out.contains("from `./inner.mts`"));
}

#[test]
fn md_format_import_uses_resolved_path() {
    let mut args = fixture_args(vec!["src/import-util.mts"], Format::Md);
    args.include = Include::Both;
    let out = run_capture(args);
    assert!(out.contains("from `src/util.mts`"));
    assert!(!out.contains("from `./util.mts`"));
}

#[test]
fn md_format_multiple_files_lists_root_paths_under_heading() {
    let out = run_capture(fixture_args(vec!["src/a.mts", "src/b.mts"], Format::Md));
    assert!(out.contains("# 2 files"));
    assert!(out.contains("- `src/a.mts`"));
    assert!(out.contains("- `src/b.mts`"));
}

#[test]
fn human_format_import_uses_resolved_path() {
    let mut args = fixture_args(vec!["src/import-util.mts"], Format::Human);
    args.include = Include::Both;
    let out = run_capture(args);
    assert!(out.contains("from src/util.mts"));
    assert!(!out.contains("from ./util.mts"));
}

#[test]
fn export_kind_enum_serializes_as_enum() {
    let out = run_capture(fixture_args(vec!["src/enum.mts"], Format::Json));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["files"][0]["exports"][0]["kind"], "enum");
    assert_eq!(v["files"][0]["exports"][0]["name"], "Color");
}
