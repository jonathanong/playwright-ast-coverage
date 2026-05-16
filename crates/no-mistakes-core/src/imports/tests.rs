use super::{collect_imports, collect_imports_from_program, resolve_import};
use crate::ast;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn fixture(category: &str, name: &str, file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(category)
        .join(name)
        .join(file)
}

#[test]
fn resolve_import_no_parent_returns_none() {
    // A root path `/` has no parent — `current_file.parent()` returns None,
    // causing the `?` at line 13 to return None from the function.
    let result = resolve_import(Path::new("/"), "./sibling");
    assert!(result.is_none());
}

#[test]
fn collect_imports_nonexistent_path_returns_error() {
    // canonicalize() fails on a nonexistent path, exercising line 50 error branch.
    let mut cache = HashMap::new();
    let result = collect_imports(Path::new("/nonexistent/path/file.ts"), &mut cache);
    assert!(result.is_err());
}

#[test]
#[cfg(unix)]
fn collect_imports_unreadable_file_returns_error() {
    use std::os::unix::fs::PermissionsExt;
    // Create a file then remove read permissions so read_to_string fails (line 55).
    let dir = tempdir().unwrap();
    let file = dir.path().join("unreadable.ts");
    let fixture_file = fixture("no-mistakes-core-imports", "unreadable-file", "main.ts");
    std::fs::copy(&fixture_file, &file).unwrap();
    let mut perms = file.metadata().unwrap().permissions();
    perms.set_mode(0o000);
    std::fs::set_permissions(&file, perms).unwrap();

    let mut cache = HashMap::new();
    let result = collect_imports(&file, &mut cache);
    assert!(result.is_err());
}

#[test]
fn collect_imports_parse_error_returns_error() {
    // A file with a parse error causes ast::with_program to return Err,
    // which propagates through the ? at line 60 of collect_imports.
    let file = fixture("no-mistakes-core-imports", "parse-error", "bad.ts");
    let mut cache = HashMap::new();
    let result = collect_imports(&file, &mut cache);
    assert!(result.is_err());
}

#[test]
fn collect_imports_from_program_unresolvable_import() {
    // A relative import that doesn't exist on disk causes resolve_import to return None,
    // which means the import is skipped (exercises the None branch).
    let abs_path = fixture("no-mistakes-core-imports", "unresolvable-import", "main.ts")
        .canonicalize()
        .unwrap();
    let source = std::fs::read_to_string(&abs_path).unwrap();
    let mut cache = HashMap::new();
    let imports = ast::with_program(&abs_path, &source, |program, _| {
        collect_imports_from_program(&abs_path, program, &mut cache)
    })
    .unwrap();

    assert!(imports.is_empty(), "unresolvable import should be skipped");
}

#[test]
fn collect_imports_from_program_unresolvable_export_named() {
    // A re-export from a nonexistent module — exercises the None branch in ExportNamedDeclaration.
    let abs_path = fixture(
        "no-mistakes-core-imports",
        "unresolvable-export-named",
        "main.ts",
    )
    .canonicalize()
    .unwrap();
    let source = std::fs::read_to_string(&abs_path).unwrap();
    let mut cache = HashMap::new();
    let imports = ast::with_program(&abs_path, &source, |program, _| {
        collect_imports_from_program(&abs_path, program, &mut cache)
    })
    .unwrap();

    assert!(imports.is_empty());
}

#[test]
fn collect_imports_from_program_unresolvable_export_all() {
    // An export-all from a nonexistent module — exercises the None branch in ExportAllDeclaration.
    let abs_path = fixture(
        "no-mistakes-core-imports",
        "unresolvable-export-all",
        "main.ts",
    )
    .canonicalize()
    .unwrap();
    let source = std::fs::read_to_string(&abs_path).unwrap();
    let mut cache = HashMap::new();
    let imports = ast::with_program(&abs_path, &source, |program, _| {
        collect_imports_from_program(&abs_path, program, &mut cache)
    })
    .unwrap();

    assert!(imports.is_empty());
}
