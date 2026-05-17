use super::super::*;
use tempfile::TempDir;

fn pkg(content: &str) -> PackageJson {
    serde_json::from_str(content).unwrap()
}

#[test]
fn workspace_globs_and_expansion_cover_empty_and_outside_paths() {
    let dir = TempDir::new().unwrap();
    super::write(&dir.path().join("package.json"), r#"{"name":"root"}"#);
    assert!(load_workspace_globs(dir.path()).unwrap().is_empty());
    assert!(expand_workspace_globs(dir.path(), &["[".to_string()]).is_empty());
    assert!(expand_workspace_globs_from_files(dir.path(), &["[".to_string()], &[]).is_empty());

    let files = vec![
        dir.path().join("packages/app/package.json"),
        dir.path().join("packages/ignored/package.json"),
        dir.path().join("../outside/package.json"),
    ];
    let dirs = expand_workspace_globs_from_files(
        dir.path(),
        &["packages/*".to_string(), "!packages/ignored".to_string()],
        &files,
    );
    assert_eq!(dirs, vec![dir.path().join("packages/app")]);
}

#[test]
fn resolve_entry_covers_module_main_types_and_fallbacks() {
    let dir = TempDir::new().unwrap();

    let module_file = dir.path().join("dist/module.mjs");
    super::write(&module_file, "");
    assert_eq!(
        resolve_entry(dir.path(), &pkg(r#"{"module":"dist/module.mjs"}"#)),
        Some(module_file)
    );

    let main_file = dir.path().join("dist/main.js");
    super::write(&main_file, "");
    assert_eq!(
        resolve_entry(dir.path(), &pkg(r#"{"main":"dist/main.js"}"#)),
        Some(main_file)
    );

    let types_file = dir.path().join("dist/types.d.ts");
    super::write(&types_file, "");
    assert_eq!(
        resolve_entry(dir.path(), &pkg(r#"{"types":"dist/types.d.ts"}"#)),
        Some(types_file)
    );

    let fallback = dir.path().join("src/index.ts");
    super::write(&fallback, "");
    assert_eq!(resolve_entry(dir.path(), &pkg(r#"{}"#)), Some(fallback));
}

#[test]
fn export_and_specifier_helpers_cover_unmatched_shapes() {
    let exports = serde_json::json!({
        "./bad/*/again/*": "./bad/*.mts",
        "./missing/*": null,
        "./exact": {"require": "./exact.cjs"}
    });
    assert_eq!(resolve_export_subpath(&exports, "./bad/x/again/y"), None);
    assert_eq!(resolve_export_subpath(&exports, "./missing/x"), None);
    assert_eq!(
        resolve_export_subpath(&exports, "./exact"),
        Some("./exact.cjs".to_string())
    );

    assert_eq!(package_name_and_subpath("./local"), None);
    assert_eq!(package_name_and_subpath("@scope"), None);
}

#[test]
fn try_resolve_appends_supported_extension() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("entry.ts");
    super::write(&file, "");
    assert_eq!(try_resolve(&dir.path().join("entry")), Some(file));
}

#[test]
fn resolve_entry_covers_exports_and_extension_fallback_returns() {
    let dir = crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/workspaces-entries/pkg"),
    );

    assert_eq!(
        resolve_entry(
            &dir,
            &pkg(r#"{"exports":{".":{"import":"./exported/index.ts"}}}"#)
        ),
        Some(dir.join("exported/index.ts"))
    );
    assert_eq!(
        resolve_entry(&dir, &pkg(r#"{"module":"dist/module"}"#)),
        Some(dir.join("dist/module.mjs"))
    );
    assert_eq!(
        resolve_entry(&dir, &pkg(r#"{"main":"dist/main"}"#)),
        Some(dir.join("dist/main.js"))
    );
    assert_eq!(
        resolve_entry(&dir, &pkg(r#"{"types":"dist/types.d.ts"}"#)),
        Some(dir.join("dist/types.d.ts"))
    );
    let default_export_dir =
        crate::codebase::ts_resolver::normalize_path(&dir.join("../default-export"));
    assert_eq!(
        resolve_entry(
            &default_export_dir,
            &pkg(r#"{"exports":{"default":"./index.ts"}}"#)
        ),
        Some(default_export_dir.join("index.ts"))
    );
    let types_only_dir = crate::codebase::ts_resolver::normalize_path(&dir.join("../types-only"));
    assert_eq!(
        resolve_entry(&types_only_dir, &pkg(r#"{"types":"index.d.ts"}"#)),
        Some(types_only_dir.join("index.d.ts"))
    );
    assert_eq!(try_resolve(&dir.join("missing-entry")), None);
}

#[test]
fn fixture_load_covers_entry_resolution_branches() {
    let root = crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/workspaces-entries"),
    );

    let map = load(&root).unwrap();

    assert_eq!(
        super::package_entry(&map, "@fixtures/exported"),
        Some(&root.join("pkg/exported/index.ts"))
    );
    assert_eq!(
        super::package_entry(&map, "@fixtures/default-export"),
        Some(&root.join("default-export/index.ts"))
    );
    assert_eq!(
        super::package_entry(&map, "@fixtures/types-only"),
        Some(&root.join("types-only/index.d.ts"))
    );
    assert_eq!(
        super::package_entry(&map, "@fixtures/main-only"),
        Some(&root.join("main-only/index.js"))
    );
}
