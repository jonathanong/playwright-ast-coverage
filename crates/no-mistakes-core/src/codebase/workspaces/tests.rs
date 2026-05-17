use super::*;
use tempfile::TempDir;

mod extra;

fn write(path: &Path, content: &str) {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

fn package_entry<'a>(map: &'a WorkspaceMap, name: &str) -> Option<&'a PathBuf> {
    map.packages
        .iter()
        .find(|package| package.name == name)
        .and_then(|package| package.entry.as_ref())
}

// ── load with no package.json ─────────────────────────────────────────

#[test]
fn no_package_json_returns_empty() {
    let dir = TempDir::new().unwrap();
    let map = load(dir.path()).unwrap();
    assert!(map.packages.is_empty());
}

#[test]
fn invalid_workspace_glob_returns_no_dirs() {
    let dir = TempDir::new().unwrap();
    write(&dir.path().join("package.json"), r#"{"workspaces": ["["]}"#);

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

// ── Workspace package entries ─────────────────────────────────────────

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
    assert_eq!(package_entry(&map, "@x/api"), Some(&entry));
}

#[test]
fn resolve_package_missing_returns_none() {
    let map = WorkspaceMap::default();
    assert!(package_entry(&map, "@x/missing").is_none());
}

#[test]
fn resolve_specifier_rejects_relative_and_missing_packages() {
    let map = WorkspaceMap::default();

    assert_eq!(map.resolve_specifier("./local"), None);
    assert_eq!(map.resolve_specifier("/abs"), None);
    assert_eq!(map.resolve_specifier("@missing/pkg/subpath"), None);
}

#[test]
fn resolve_specifier_rejects_unprefixed_subpath_without_exports() {
    let dir = TempDir::new().unwrap();
    let map = WorkspaceMap {
        packages: vec![WorkspacePackage {
            name: "@x/api".to_string(),
            dir: dir.path().to_path_buf(),
            entry: None,
            exports: None,
        }],
    };

    assert_eq!(map.resolve_specifier("@x/api"), None);
    assert_eq!(map.packages[0].resolve_subpath("src/public"), None);
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
fn prefers_exports_over_other_entry_fields() {
    let dir = TempDir::new().unwrap();
    write(&dir.path().join("dist/index.js"), "");
    write(&dir.path().join("esm.mts"), "");
    let pkg = PackageJson {
        exports: Some(serde_json::json!("./dist/index.js")),
        module: Some("esm.mts".to_string()),
        main: Some("cjs.js".to_string()),
        ..Default::default()
    };

    let entry = resolve_entry(dir.path(), &pkg).unwrap();

    assert!(entry.ends_with("dist/index.js"));
}

#[test]
fn falls_back_to_main_then_types() {
    let main_dir = TempDir::new().unwrap();
    write(&main_dir.path().join("cjs.js"), "");
    let main_pkg = PackageJson {
        main: Some("cjs.js".to_string()),
        types: Some("index.d.ts".to_string()),
        ..Default::default()
    };
    assert!(resolve_entry(main_dir.path(), &main_pkg)
        .unwrap()
        .ends_with("cjs.js"));

    let types_dir = TempDir::new().unwrap();
    write(&types_dir.path().join("index.d.ts"), "");
    let types_pkg = PackageJson {
        types: Some("index.d.ts".to_string()),
        ..Default::default()
    };
    assert!(resolve_entry(types_dir.path(), &types_pkg)
        .unwrap()
        .ends_with("index.d.ts"));
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

#[test]
fn resolve_entry_returns_none_when_no_candidates_exist() {
    let dir = TempDir::new().unwrap();
    let pkg = PackageJson {
        exports: Some(serde_json::json!("./missing.mts")),
        module: Some("missing-module.mts".to_string()),
        main: Some("missing-main.mts".to_string()),
        types: Some("missing.d.ts".to_string()),
        ..Default::default()
    };

    assert_eq!(resolve_entry(dir.path(), &pkg), None);
}

#[test]
fn resolve_entry_continues_when_exports_have_no_entry_path() {
    let dir = TempDir::new().unwrap();
    write(&dir.path().join("main.mts"), "");
    let pkg = PackageJson {
        exports: Some(serde_json::json!({"browser": false})),
        main: Some("main.mts".to_string()),
        ..Default::default()
    };

    assert!(resolve_entry(dir.path(), &pkg)
        .unwrap()
        .ends_with("main.mts"));
}

#[test]
fn load_package_ignores_invalid_json_and_missing_name() {
    let invalid = TempDir::new().unwrap();
    write(&invalid.path().join("package.json"), "{ invalid");
    assert!(load_package(invalid.path()).unwrap().is_none());

    let unnamed = TempDir::new().unwrap();
    write(&unnamed.path().join("package.json"), r#"{"name": ""}"#);
    assert!(load_package(unnamed.path()).unwrap().is_none());
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
fn exports_to_entry_path_unsupported_values_return_none() {
    assert_eq!(exports_to_entry_path(&serde_json::json!(false)), None);
    assert_eq!(
        exports_to_entry_path(&serde_json::json!({"browser": false})),
        None
    );
}

#[test]
fn resolve_export_subpath_rejects_non_object_and_multi_star_patterns() {
    assert_eq!(
        resolve_export_subpath(&serde_json::json!("./index.mts"), "./anything"),
        None
    );
    assert_eq!(
        resolve_export_subpath(&serde_json::json!({"./*/*": "./src/*.mts"}), "./a/b"),
        None
    );
    assert_eq!(
        resolve_export_subpath(&serde_json::json!({"./*": "./src/file.mts"}), "./a"),
        None
    );
}

#[test]
fn export_pattern_sort_uses_pattern_name_as_tiebreaker() {
    let b_pattern = "./b/*".to_string();
    let a_pattern = "./a/*".to_string();
    let b_value = serde_json::json!("./b/*.mts");
    let a_value = serde_json::json!("./a/*.mts");
    let mut patterns = [(&b_pattern, &b_value, 4), (&a_pattern, &a_value, 4)];

    patterns.sort_by(compare_export_patterns);

    assert_eq!(patterns[0].0, "./a/*");
}

#[test]
fn resolve_specifier_falls_back_to_subpath_without_exports() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("src/public.mts");
    write(&target, "");
    let map = WorkspaceMap {
        packages: vec![WorkspacePackage {
            name: "@x/api".to_string(),
            dir: dir.path().to_path_buf(),
            entry: None,
            exports: None,
        }],
    };

    assert_eq!(map.resolve_specifier("@x/api/src/public"), Some(target));
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

// ── workspace-conditional-exports fixture ──────────────────────────────────────────

#[test]
fn fixture_workspace_conditional_exports() {
    // fixtures/workspace-conditional-exports/shared/session-token/package.json uses
    // the conditional exports object form: { ".": { "types", "import", "default" } }
    let pkg_json = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../fixtures/codebase-analysis/workspace-conditional-exports/shared/session-token/package.json",
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
