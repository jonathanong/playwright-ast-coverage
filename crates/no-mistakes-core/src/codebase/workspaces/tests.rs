use super::*;
use tempfile::TempDir;

fn write(path: &Path, content: &str) {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

// ── load with no package.json ─────────────────────────────────────────

#[test]
fn no_package_json_returns_empty() {
    let dir = TempDir::new().unwrap();
    let map = load(dir.path()).unwrap();
    assert!(map.packages.is_empty());
}

// ── load with workspaces as array ─────────────────────────────────────

#[test]
fn loads_workspace_array() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    write(
        &root.join("package.json"),
        r#"{"workspaces": ["packages/*"]}"#,
    );
    write(
        &root.join("packages/api/package.json"),
        r#"{"name": "@x/api", "main": "src/index.mts"}"#,
    );
    write(&root.join("packages/api/src/index.mts"), "export {};");

    let map = load(root).unwrap();
    assert_eq!(map.packages.len(), 1);
    assert_eq!(map.packages[0].name, "@x/api");
    assert!(map.packages[0].entry.is_some());
}

// ── load with workspaces as object ────────────────────────────────────

#[test]
fn loads_workspace_object_form() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    write(
        &root.join("package.json"),
        r#"{"workspaces": {"packages": ["packages/*"]}}"#,
    );
    write(
        &root.join("packages/web/package.json"),
        r#"{"name": "@x/web", "main": "src/index.tsx"}"#,
    );
    write(&root.join("packages/web/src/index.tsx"), "export {};");

    let map = load(root).unwrap();
    assert_eq!(map.packages.len(), 1);
    assert_eq!(map.packages[0].name, "@x/web");
}

#[test]
fn loads_pnpm_workspace_yaml_when_package_json_has_no_workspaces() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    write(&root.join("package.json"), r#"{"name": "root"}"#);
    write(
        &root.join("pnpm-workspace.yaml"),
        "packages:\n  - packages/*\n",
    );
    write(
        &root.join("packages/api/package.json"),
        r#"{"name": "@x/api", "main": "src/index.mts"}"#,
    );
    write(&root.join("packages/api/src/index.mts"), "export {};");

    let map = load(root).unwrap();
    assert_eq!(map.packages.len(), 1);
    assert_eq!(map.packages[0].name, "@x/api");
}

#[test]
fn pnpm_workspace_yaml_takes_precedence_over_package_json_workspaces() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    write(
        &root.join("package.json"),
        r#"{"workspaces": ["npm-packages/*"]}"#,
    );
    write(
        &root.join("pnpm-workspace.yaml"),
        "packages:\n  - pnpm-packages/*\n",
    );
    write(
        &root.join("npm-packages/api/package.json"),
        r#"{"name": "@x/npm"}"#,
    );
    write(
        &root.join("pnpm-packages/api/package.json"),
        r#"{"name": "@x/pnpm"}"#,
    );

    let map = load(root).unwrap();
    assert_eq!(map.packages.len(), 1);
    assert_eq!(map.packages[0].name, "@x/pnpm");
}

#[test]
fn pnpm_workspace_without_packages_includes_direct_subdirectories() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    write(&root.join("pnpm-workspace.yaml"), "{}\n");
    write(&root.join("api/package.json"), r#"{"name": "@x/api"}"#);
    write(
        &root.join("api/fixtures/nested/package.json"),
        r#"{"name": "@x/nested"}"#,
    );

    let map = load(root).unwrap();
    assert_eq!(map.packages.len(), 1);
    assert_eq!(map.packages[0].name, "@x/api");
}

#[test]
fn pnpm_workspace_exclusion_globs_remove_loaded_packages() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    write(
        &root.join("pnpm-workspace.yaml"),
        "packages:\n  - packages/**\n  - '!packages/**/fixtures/**'\n",
    );
    write(
        &root.join("packages/group/foo/package.json"),
        r#"{"name": "@x/foo"}"#,
    );
    write(
        &root.join("packages/group/foo/fixtures/bar/package.json"),
        r#"{"name": "@x/bar"}"#,
    );

    let map = load(root).unwrap();
    assert_eq!(map.packages.len(), 1);
    assert_eq!(map.packages[0].name, "@x/foo");
}

// ── WorkspaceMap::resolve_package ─────────────────────────────────────

#[test]
fn resolve_package_finds_by_name() {
    let dir = TempDir::new().unwrap();
    let entry = dir.path().join("src/index.mts");
    write(&entry, "");
    let map = WorkspaceMap {
        packages: vec![WorkspacePackage {
            name: "@x/api".to_string(),
            dir: dir.path().to_path_buf(),
            entry: Some(entry.clone()),
            exports: None,
        }],
    };
    assert_eq!(map.resolve_package("@x/api"), Some(&entry));
}

#[test]
fn resolve_package_missing_returns_none() {
    let map = WorkspaceMap::default();
    assert!(map.resolve_package("@x/missing").is_none());
}

// ── entry resolution order ────────────────────────────────────────────

#[test]
fn prefers_module_over_main() {
    let dir = TempDir::new().unwrap();
    write(&dir.path().join("esm.mts"), "");
    write(&dir.path().join("cjs.js"), "");
    let pkg = PackageJson {
        name: Some("pkg".to_string()),
        module: Some("esm.mts".to_string()),
        main: Some("cjs.js".to_string()),
        ..Default::default()
    };
    let entry = resolve_entry(dir.path(), &pkg);
    assert!(entry.unwrap().to_str().unwrap().ends_with("esm.mts"));
}

#[test]
fn falls_back_to_src_index_mts() {
    let dir = TempDir::new().unwrap();
    write(&dir.path().join("src/index.mts"), "");
    let pkg = PackageJson::default();
    let entry = resolve_entry(dir.path(), &pkg);
    assert!(entry.is_some());
    assert!(entry.unwrap().ends_with("src/index.mts"));
}

// ── multiple workspaces ───────────────────────────────────────────────

#[test]
fn loads_multiple_packages() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    write(
        &root.join("package.json"),
        r#"{"workspaces": ["packages/*"]}"#,
    );
    for name in &["api", "web", "shared"] {
        write(
            &root.join(format!("packages/{name}/package.json")),
            &format!(r#"{{"name": "@x/{name}", "main": "src/index.mts"}}"#),
        );
        write(
            &root.join(format!("packages/{name}/src/index.mts")),
            "export {};",
        );
    }

    let map = load(root).unwrap();
    assert_eq!(map.packages.len(), 3);
}

// ── exports_to_entry_path ─────────────────────────────────────────────

#[test]
fn exports_conditional_object_form_returns_import_field() {
    let val = serde_json::json!({".": {"import": "./dist/index.mjs", "types": "./index.d.ts"}});
    assert_eq!(
        exports_to_entry_path(&val),
        Some("./dist/index.mjs".to_string())
    );
}

#[test]
fn exports_string_form() {
    let val = serde_json::Value::String("./index.mts".to_string());
    assert_eq!(exports_to_entry_path(&val), Some("./index.mts".to_string()));
}

#[test]
fn exports_dot_string_form() {
    let val = serde_json::json!({".": "./dist/index.js"});
    assert_eq!(
        exports_to_entry_path(&val),
        Some("./dist/index.js".to_string())
    );
}

#[test]
fn resolve_specifier_supports_exact_subpath_export() {
    let dir = TempDir::new().unwrap();
    let entry = dir.path().join("src/public.mts");
    write(&entry, "");
    let map = WorkspaceMap {
        packages: vec![WorkspacePackage {
            name: "@x/api".to_string(),
            dir: dir.path().to_path_buf(),
            entry: None,
            exports: Some(serde_json::json!({"./public": "./src/public.mts"})),
        }],
    };
    assert_eq!(map.resolve_specifier("@x/api/public"), Some(entry));
}

#[test]
fn resolve_specifier_supports_single_star_subpath_export() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("enqueues.mts");
    write(&target, "");
    let map = WorkspaceMap {
        packages: vec![WorkspacePackage {
            name: "@systems/foo".to_string(),
            dir: dir.path().to_path_buf(),
            entry: None,
            exports: Some(serde_json::json!({"./*": {"types": "./*.mts", "import": "./*.mts"}})),
        }],
    };
    assert_eq!(map.resolve_specifier("@systems/foo/enqueues"), Some(target));
}

#[test]
fn resolve_specifier_does_not_fallback_when_exports_are_defined() {
    let dir = TempDir::new().unwrap();
    write(&dir.path().join("internal.mts"), "");
    let map = WorkspaceMap {
        packages: vec![WorkspacePackage {
            name: "@x/api".to_string(),
            dir: dir.path().to_path_buf(),
            entry: None,
            exports: Some(serde_json::json!({"./public": "./public.mts"})),
        }],
    };
    assert_eq!(map.resolve_specifier("@x/api/internal"), None);
}

#[test]
fn resolve_specifier_continues_after_unsupported_matching_pattern() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("src/foo/value.mts");
    write(&target, "");
    let map = WorkspaceMap {
        packages: vec![WorkspacePackage {
            name: "@x/api".to_string(),
            dir: dir.path().to_path_buf(),
            entry: None,
            exports: Some(serde_json::json!({
                "./foo/*": null,
                "./*": "./src/*.mts"
            })),
        }],
    };
    assert_eq!(map.resolve_specifier("@x/api/foo/value"), Some(target));
}

#[test]
fn resolve_specifier_prefers_more_specific_star_export() {
    let dir = TempDir::new().unwrap();
    let specific = dir.path().join("specific/value.mts");
    let broad = dir.path().join("broad/foo/bar/value.mts");
    write(&specific, "");
    write(&broad, "");
    let map = WorkspaceMap {
        packages: vec![WorkspacePackage {
            name: "@x/api".to_string(),
            dir: dir.path().to_path_buf(),
            entry: None,
            exports: Some(serde_json::json!({
                "./foo/*": "./broad/foo/*.mts",
                "./foo/bar/*": "./specific/*.mts"
            })),
        }],
    };
    assert_eq!(
        map.resolve_specifier("@x/api/foo/bar/value"),
        Some(specific)
    );
}

// ── filaments-workspace fixture ──────────────────────────────────────────

#[test]
fn fixture_filaments_workspace_session_jwt_conditional_exports() {
    // fixtures/filaments-workspace/ts-shared/session-jwt/package.json uses
    // the conditional exports object form: { ".": { "types", "import", "default" } }
    let pkg_json = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../fixtures/codebase-analysis/filaments-workspace/ts-shared/session-jwt/package.json",
    ))
    .unwrap();
    let pkg: serde_json::Value = serde_json::from_str(&pkg_json).unwrap();
    let exports = &pkg["exports"];
    // Should resolve to the "import" field of the "." key
    let entry = exports_to_entry_path(exports);
    assert_eq!(
        entry,
        Some("./index.mts".to_string()),
        "conditional exports should resolve import field, got: {entry:?}"
    );
}

// ── package without package.json ignored ────────────────────────────

#[test]
fn dir_without_package_json_ignored() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        &root.join("package.json"),
        r#"{"workspaces": ["packages/*"]}"#,
    );
    // Create dir without package.json.
    std::fs::create_dir_all(root.join("packages/no-pkg")).unwrap();
    let map = load(root).unwrap();
    assert!(map.packages.is_empty());
}
