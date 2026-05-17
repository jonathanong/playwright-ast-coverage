use super::*;
use std::collections::HashSet;
use tempfile::TempDir;

fn write(path: &Path, content: &str) {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

fn make_tsconfig(dir: &Path, paths_json: &str) -> TsConfig {
    let content = format!(r#"{{"compilerOptions": {{"paths": {}}}}}"#, paths_json);
    let p = dir.join("tsconfig.json");
    write(&p, &content);
    load_tsconfig(&p).unwrap()
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis/ts-resolver")
        .join(name)
}

// ── load_tsconfig ─────────────────────────────────────────────────────

#[test]
fn load_tsconfig_parses_paths() {
    let dir = TempDir::new().unwrap();
    let tc = make_tsconfig(dir.path(), r#"{"@utils/*": ["./utils/*"]}"#);
    assert_eq!(tc.paths.len(), 1);
    assert_eq!(tc.paths[0].0, "@utils/*");
}

#[test]
fn load_tsconfig_empty_returns_defaults() {
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("tsconfig.json");
    write(&p, "{}");
    let tc = load_tsconfig(&p).unwrap();
    assert!(tc.paths.is_empty());
}

#[test]
fn load_tsconfig_invalid_json_errors() {
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("tsconfig.json");
    write(&p, "{ bad json }");
    assert!(load_tsconfig(&p).is_err());
}

#[test]
fn load_tsconfig_missing_file_errors() {
    let dir = TempDir::new().unwrap();
    assert!(load_tsconfig(&dir.path().join("tsconfig.json")).is_err());
}

// ── find_tsconfig ─────────────────────────────────────────────────────

#[test]
fn find_tsconfig_finds_in_dir() {
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("tsconfig.json");
    write(&p, "{}");
    assert_eq!(find_tsconfig(dir.path()), Some(p));
}

#[test]
fn find_tsconfig_finds_in_parent() {
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("tsconfig.json");
    write(&p, "{}");
    let child = dir.path().join("sub").join("dir");
    std::fs::create_dir_all(&child).unwrap();
    assert_eq!(find_tsconfig(&child), Some(p));
}

#[test]
fn find_tsconfig_finds_from_file() {
    let dir = TempDir::new().unwrap();
    let tsc = dir.path().join("tsconfig.json");
    write(&tsc, "{}");
    let file = dir.path().join("src").join("main.mts");
    write(&file, "");
    assert_eq!(find_tsconfig(&file), Some(tsc));
}

// ── resolve_import — relative ─────────────────────────────────────────

#[test]
fn resolves_relative_with_extension() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("src").join("utils.mts");
    write(&target, "");
    let importer = dir.path().join("src").join("main.mts");
    let tc = TsConfig {
        dir: dir.path().to_path_buf(),
        paths: vec![],
        paths_dir: dir.path().to_path_buf(),
        base_url: None,
    };
    assert_eq!(resolve_import("./utils.mts", &importer, &tc), Some(target));
}

#[test]
fn resolves_relative_no_ext_tries_mts() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("src").join("utils.mts");
    write(&target, "");
    let importer = dir.path().join("src").join("main.mts");
    let tc = TsConfig {
        dir: dir.path().to_path_buf(),
        paths: vec![],
        paths_dir: dir.path().to_path_buf(),
        base_url: None,
    };
    assert_eq!(resolve_import("./utils", &importer, &tc), Some(target));
}

#[test]
fn resolves_relative_no_ext_falls_back_to_ts() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("src").join("utils.ts");
    write(&target, "");
    let importer = dir.path().join("src").join("main.mts");
    let tc = TsConfig {
        dir: dir.path().to_path_buf(),
        paths: vec![],
        paths_dir: dir.path().to_path_buf(),
        base_url: None,
    };
    assert_eq!(resolve_import("./utils", &importer, &tc), Some(target));
}

#[test]
fn resolves_relative_dotted_stem_by_appending_known_extension() {
    let root = fixture("dotted-stem");
    let importer = root.join("src/main.mts");
    let target = normalize_path(&root.join("src/button.stories.tsx"));
    let tc = TsConfig {
        dir: root.clone(),
        paths: vec![],
        paths_dir: root,
        base_url: None,
    };
    assert_eq!(
        resolve_import("./button.stories", &importer, &tc),
        Some(target)
    );
}

#[test]
fn resolves_relative_parent() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("lib.mts");
    write(&target, "");
    // Create the src directory so ../lib.mts resolves through an existing parent.
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let importer = src.join("main.mts");
    let tc = TsConfig {
        dir: dir.path().to_path_buf(),
        paths: vec![],
        paths_dir: dir.path().to_path_buf(),
        base_url: None,
    };
    assert_eq!(resolve_import("../lib.mts", &importer, &tc), Some(target));
}

#[test]
fn resolves_relative_index_fallback() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("src").join("utils").join("index.mts");
    write(&target, "");
    let importer = dir.path().join("src").join("main.mts");
    let tc = TsConfig {
        dir: dir.path().to_path_buf(),
        paths: vec![],
        paths_dir: dir.path().to_path_buf(),
        base_url: None,
    };
    assert_eq!(resolve_import("./utils", &importer, &tc), Some(target));
}

#[test]
fn relative_nonexistent_returns_none() {
    let dir = TempDir::new().unwrap();
    let importer = dir.path().join("main.mts");
    let tc = TsConfig {
        dir: dir.path().to_path_buf(),
        paths: vec![],
        paths_dir: dir.path().to_path_buf(),
        base_url: None,
    };
    assert!(resolve_import("./ghost", &importer, &tc).is_none());
}

// ── resolve_import — aliases ──────────────────────────────────────────

#[test]
fn resolves_alias_exact() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("lib").join("core.mts");
    write(&target, "");
    let tc = make_tsconfig(dir.path(), r#"{"@core": ["./lib/core"]}"#);
    let importer = dir.path().join("main.mts");
    assert_eq!(resolve_import("@core", &importer, &tc), Some(target));
}

#[test]
fn resolves_alias_wildcard() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("utils").join("helpers.mts");
    write(&target, "");
    let tc = make_tsconfig(dir.path(), r#"{"@utils/*": ["./utils/*"]}"#);
    let importer = dir.path().join("main.mts");
    assert_eq!(
        resolve_import("@utils/helpers", &importer, &tc),
        Some(target)
    );
}

#[test]
fn alias_wildcard_with_subpath() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("systems").join("emails").join("queues.mts");
    write(&target, "");
    let tc = make_tsconfig(dir.path(), r#"{"@systems/*": ["./systems/*"]}"#);
    let importer = dir.path().join("main.mts");
    assert_eq!(
        resolve_import("@systems/emails/queues", &importer, &tc),
        Some(target)
    );
}

#[test]
fn alias_nonexistent_returns_none() {
    let dir = TempDir::new().unwrap();
    let tc = make_tsconfig(dir.path(), r#"{"@utils/*": ["./utils/*"]}"#);
    let importer = dir.path().join("main.mts");
    assert!(resolve_import("@utils/ghost", &importer, &tc).is_none());
}

#[test]
fn bare_npm_returns_none() {
    let dir = TempDir::new().unwrap();
    let tc = TsConfig {
        dir: dir.path().to_path_buf(),
        paths: vec![],
        paths_dir: dir.path().to_path_buf(),
        base_url: None,
    };
    let importer = dir.path().join("main.mts");
    assert!(resolve_import("express", &importer, &tc).is_none());
    assert!(resolve_import("node:path", &importer, &tc).is_none());
}

#[test]
fn catch_all_nonexistent_returns_none() {
    let dir = TempDir::new().unwrap();
    let tc = make_tsconfig(dir.path(), r#"{"*": ["./*"]}"#);
    let importer = dir.path().join("main.mts");
    assert!(resolve_import("some-npm-pkg", &importer, &tc).is_none());
}

#[test]
fn import_resolver_uses_visible_file_set() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("src").join("utils.mts");
    let importer = dir.path().join("src").join("main.mts");
    let tc = TsConfig {
        dir: dir.path().to_path_buf(),
        paths: vec![],
        paths_dir: dir.path().to_path_buf(),
        base_url: None,
    };
    let visible: HashSet<PathBuf> = [target.clone()].into();
    let resolver = ImportResolver::new(&tc).with_visible(&visible);

    assert_eq!(resolver.resolve("./utils", &importer), Some(target));
}

#[test]
fn import_resolver_cache_reuses_present_result() {
    let dir = TempDir::new().unwrap();
    let target = normalize_path(&dir.path().join("src").join("utils.mts"));
    let importer = dir.path().join("src").join("main.mts");
    let tc = TsConfig {
        dir: dir.path().to_path_buf(),
        paths: vec![],
        paths_dir: dir.path().to_path_buf(),
        base_url: None,
    };
    let visible: HashSet<PathBuf> = [target.clone()].into();
    let resolver = ImportResolver::new(&tc).with_visible(&visible);

    assert_eq!(resolver.resolve("./utils", &importer), Some(target.clone()));
    assert_eq!(resolver.resolve("./utils", &importer), Some(target));
    assert!(resolver.resolve("./utils.mts", &importer).is_some());
    assert!(resolver.resolve("./missing.mts", &importer).is_none());
}

#[test]
fn import_resolver_cache_preserves_missing_result() {
    let dir = TempDir::new().unwrap();
    let importer = dir.path().join("src").join("main.mts");
    let target = dir.path().join("src").join("utils.mts");
    let tc = TsConfig {
        dir: dir.path().to_path_buf(),
        paths: vec![],
        paths_dir: dir.path().to_path_buf(),
        base_url: None,
    };
    let resolver = ImportResolver::new(&tc);

    assert!(resolver.resolve("./utils", &importer).is_none());
    write(&target, "");
    assert!(resolver.resolve("./utils", &importer).is_none());
}

// ── match_alias ───────────────────────────────────────────────────────

#[test]
fn match_alias_exact() {
    assert_eq!(match_alias("@core", "@core"), Some(String::new()));
    assert_eq!(match_alias("@core", "@other"), None);
}

#[test]
fn match_alias_wildcard() {
    assert_eq!(match_alias("@u/*", "@u/foo"), Some("foo".to_string()));
    assert_eq!(match_alias("@u/*", "@v/foo"), None);
}

#[test]
fn match_alias_wildcard_subpath() {
    assert_eq!(
        match_alias("@sys/*", "@sys/emails/queues"),
        Some("emails/queues".to_string())
    );
}

// ── load_tsconfig extends ────────────────────────────────────────────

#[test]
fn load_tsconfig_follows_extends_relative() {
    let dir = TempDir::new().unwrap();
    let base_p = dir.path().join("tsconfig.base.json");
    write(
        &base_p,
        r#"{"compilerOptions": {"paths": {"@utils/*": ["./utils/*"]}}}"#,
    );
    let child_p = dir.path().join("tsconfig.json");
    write(&child_p, r#"{"extends": "./tsconfig.base.json"}"#);

    let tc = load_tsconfig(&child_p).unwrap();
    assert_eq!(tc.paths.len(), 1);
    assert_eq!(tc.paths[0].0, "@utils/*");
    assert_eq!(tc.paths_dir, dir.path().to_path_buf());
}

#[test]
fn load_tsconfig_follows_extends_from_subdir() {
    let root = TempDir::new().unwrap();
    let base_p = root.path().join("tsconfig.base.json");
    write(
        &base_p,
        r#"{"compilerOptions": {"paths": {"@core/*": ["./packages/core/src/*"]}}}"#,
    );
    let sub = root.path().join("apps").join("web");
    std::fs::create_dir_all(&sub).unwrap();
    let child_p = sub.join("tsconfig.json");
    write(&child_p, r#"{"extends": "../../tsconfig.base.json"}"#);

    let tc = load_tsconfig(&child_p).unwrap();
    assert_eq!(tc.dir, sub);
    assert_eq!(tc.paths_dir, root.path().to_path_buf());
    assert_eq!(tc.paths.len(), 1);
    assert_eq!(tc.paths[0].0, "@core/*");
}

#[test]
fn load_tsconfig_child_paths_override_extends() {
    let dir = TempDir::new().unwrap();
    let base_p = dir.path().join("tsconfig.base.json");
    write(
        &base_p,
        r#"{"compilerOptions": {"paths": {"@base/*": ["./base/*"]}}}"#,
    );
    let child_p = dir.path().join("tsconfig.json");
    write(
        &child_p,
        r#"{"extends": "./tsconfig.base.json", "compilerOptions": {"paths": {"@child/*": ["./child/*"]}}}"#,
    );

    let tc = load_tsconfig(&child_p).unwrap();
    assert_eq!(tc.paths.len(), 1);
    assert_eq!(tc.paths[0].0, "@child/*");
    assert_eq!(tc.paths_dir, dir.path().to_path_buf());
}

#[test]
fn load_tsconfig_inherits_base_url_without_paths() {
    let dir = TempDir::new().unwrap();
    let base_p = dir.path().join("tsconfig.base.json");
    write(&base_p, r#"{"compilerOptions": {"baseUrl": "."}}"#);
    let child_p = dir.path().join("tsconfig.json");
    write(&child_p, r#"{"extends": "./tsconfig.base.json"}"#);
    let target = dir.path().join("lib").join("thing.mts");
    write(&target, "");

    let tc = load_tsconfig(&child_p).unwrap();
    assert_eq!(tc.base_url, Some(dir.path().to_path_buf()));
    assert_eq!(
        resolve_import("lib/thing", &dir.path().join("src").join("main.mts"), &tc),
        Some(target)
    );
}

#[test]
fn load_tsconfig_child_paths_override_extends_but_inherit_base_url() {
    let dir = TempDir::new().unwrap();
    let base_p = dir.path().join("tsconfig.base.json");
    write(
        &base_p,
        r#"{"compilerOptions": {"baseUrl": ".", "paths": {"@base/*": ["./base/*"]}}}"#,
    );
    let child_p = dir.path().join("tsconfig.json");
    write(
        &child_p,
        r#"{"extends": "./tsconfig.base.json", "compilerOptions": {"paths": {"@child/*": ["./child/*"]}}}"#,
    );

    let tc = load_tsconfig(&child_p).unwrap();
    assert_eq!(tc.paths.len(), 1);
    assert_eq!(tc.paths[0].0, "@child/*");
    assert_eq!(tc.base_url, Some(dir.path().to_path_buf()));
}

#[test]
fn load_tsconfig_extends_missing_target_errors() {
    let dir = TempDir::new().unwrap();
    let child_p = dir.path().join("tsconfig.json");
    write(&child_p, r#"{"extends": "./nonexistent.json"}"#);
    assert!(load_tsconfig(&child_p).is_err());
}

#[test]
fn load_tsconfig_extends_cycle_errors() {
    let dir = TempDir::new().unwrap();
    let a_p = dir.path().join("a.json");
    let b_p = dir.path().join("b.json");
    write(&a_p, r#"{"extends": "./b.json"}"#);
    write(&b_p, r#"{"extends": "./a.json"}"#);
    assert!(load_tsconfig(&a_p).is_err());
}

#[test]
fn load_tsconfig_extends_npm_package_skipped_gracefully() {
    // npm-package extends cannot be resolved without node_modules; degrade to empty paths.
    let dir = TempDir::new().unwrap();
    let child_p = dir.path().join("tsconfig.json");
    write(&child_p, r#"{"extends": "@scope/tsconfig/base"}"#);
    let tc = load_tsconfig(&child_p).unwrap();
    assert!(tc.paths.is_empty());
}

#[test]
fn load_tsconfig_follows_extends_array() {
    let dir = TempDir::new().unwrap();
    let base_p = dir.path().join("tsconfig.base.json");
    write(
        &base_p,
        r#"{"compilerOptions": {"paths": {"@base/*": ["./base/*"]}}}"#,
    );
    let child_p = dir.path().join("tsconfig.json");
    // TS 5.0+ array extends — rightmost entry with paths wins
    write(&child_p, r#"{"extends": ["./tsconfig.base.json"]}"#);

    let tc = load_tsconfig(&child_p).unwrap();
    assert_eq!(tc.paths.len(), 1);
    assert_eq!(tc.paths[0].0, "@base/*");
    assert_eq!(tc.paths_dir, dir.path().to_path_buf());
}

#[test]
fn load_tsconfig_extends_array_inherits_paths_and_base_url_independently() {
    let dir = TempDir::new().unwrap();
    let paths_p = dir.path().join("tsconfig.paths.json");
    write(
        &paths_p,
        r#"{"compilerOptions": {"paths": {"@base/*": ["./base/*"]}}}"#,
    );
    let base_url_p = dir.path().join("tsconfig.base-url.json");
    write(&base_url_p, r#"{"compilerOptions": {"baseUrl": "."}}"#);
    let child_p = dir.path().join("tsconfig.json");
    write(
        &child_p,
        r#"{"extends": ["./tsconfig.paths.json", "./tsconfig.base-url.json"]}"#,
    );

    let tc = load_tsconfig(&child_p).unwrap();
    assert_eq!(tc.paths.len(), 1);
    assert_eq!(tc.paths[0].0, "@base/*");
    assert_eq!(tc.paths_dir, dir.path().to_path_buf());
    assert_eq!(tc.base_url, Some(dir.path().to_path_buf()));
}

#[test]
fn load_tsconfig_extends_directory_appends_tsconfig_json() {
    let dir = TempDir::new().unwrap();
    let subdir = dir.path().join("base");
    std::fs::create_dir_all(&subdir).unwrap();
    let base_p = subdir.join("tsconfig.json");
    write(
        &base_p,
        r#"{"compilerOptions": {"paths": {"@lib/*": ["./lib/*"]}}}"#,
    );
    let child_p = dir.path().join("tsconfig.json");
    write(&child_p, r#"{"extends": "./base"}"#);

    let tc = load_tsconfig(&child_p).unwrap();
    assert_eq!(tc.paths.len(), 1);
    assert_eq!(tc.paths[0].0, "@lib/*");
}

#[test]
fn load_tsconfig_extends_extensionless_file_appends_json() {
    let dir = TempDir::new().unwrap();
    let base_p = dir.path().join("base.json");
    write(
        &base_p,
        r#"{"compilerOptions": {"paths": {"@lib/*": ["./lib/*"]}}}"#,
    );
    let child_p = dir.path().join("tsconfig.json");
    write(&child_p, r#"{"extends": "./base"}"#);

    let tc = load_tsconfig(&child_p).unwrap();
    assert_eq!(tc.paths.len(), 1);
    assert_eq!(tc.paths[0].0, "@lib/*");
}

#[test]
fn load_tsconfig_extends_array_nonstring_entry_errors() {
    let dir = TempDir::new().unwrap();
    let child_p = dir.path().join("tsconfig.json");
    // TypeScript rejects non-string entries in the extends array.
    write(&child_p, r#"{"extends": ["./tsconfig.base.json", 42]}"#);
    assert!(load_tsconfig(&child_p).is_err());
}

#[test]
fn load_tsconfig_extends_nonstring_toplevel_errors() {
    let dir = TempDir::new().unwrap();
    let child_p = dir.path().join("tsconfig.json");
    // TypeScript rejects a non-string/array extends value (e.g. a number).
    write(&child_p, r#"{"extends": 123}"#);
    assert!(load_tsconfig(&child_p).is_err());
}

#[test]
fn resolver_falls_back_when_cache_is_poisoned() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("src").join("main.mts");
    let dep = dir.path().join("src").join("dep.mts");
    write(&file, "");
    write(&dep, "");
    let tc = TsConfig {
        dir: dir.path().to_path_buf(),
        paths: vec![],
        paths_dir: dir.path().to_path_buf(),
        base_url: None,
    };
    let resolver = ImportResolver::new(&tc);
    std::thread::scope(|scope| {
        let cache = &resolver.cache;
        let _ = scope
            .spawn(move || {
                let _guard = cache.lock().unwrap();
                panic!("poison resolver cache");
            })
            .join();
    });

    assert_eq!(resolver.resolve("./dep.mts", &file), Some(dep));
}

// ── normalize_path ────────────────────────────────────────────────────

#[test]
fn normalize_path_preserves_parent_dir_at_root() {
    let p = normalize_path(Path::new("/a/../../b"));
    let s = p.to_string_lossy();
    assert!(s.contains("b"), "path should still reach b: {s}");
    assert!(!s.contains("a"), "a should have been popped: {s}");
}

#[test]
fn normalize_path_double_parent_from_root() {
    let p = normalize_path(Path::new("/../../b"));
    let s = p.to_string_lossy();
    assert!(s.contains("b"));
}

#[test]
fn normalize_path_drops_current_dir_components() {
    assert_eq!(normalize_path(Path::new("./a/./b")), Path::new("a/b"));
}

#[test]
fn match_alias_captures_wildcard_segment() {
    assert_eq!(match_alias("@/*", "@/foo"), Some("foo".to_string()));
}
