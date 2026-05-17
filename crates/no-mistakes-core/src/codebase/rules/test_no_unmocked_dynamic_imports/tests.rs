use super::*;

fn fixture() -> PathBuf {
    crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/test-no-unmocked-dynamic-imports"),
    )
}

#[test]
fn fixture_reports_unmocked_transitive_and_nonliteral_dynamic_imports() {
    let root = fixture();
    let config = crate::config::v2::load_v2_config(&root, None).unwrap();
    let findings = check(&root, &config, None).unwrap();
    assert!(findings
        .iter()
        .any(|f| f.target.as_deref() == Some("src/unmocked-child.mts")));
    assert!(findings
        .iter()
        .any(|f| f.message.contains("dynamic import")));
    assert!(!findings
        .iter()
        .any(|f| f.file.contains("disabled.test.mts")));
    assert!(!findings
        .iter()
        .any(|f| f.target.as_deref() == Some("src/types.mts")));
    assert!(findings.iter().any(|f| {
        f.file == "tests/jest-setup-leak.test.mts"
            && f.target.as_deref() == Some("src/jest-setup-target.mts")
    }));
}

#[test]
fn next_line_disable_and_unresolved_import_branches_are_reported() {
    let root = fixture();
    let config = crate::config::v2::load_v2_config(&root, None).unwrap();
    let disabled = root.join("tests").join("next-line-disabled.test.mts");
    let source = std::fs::read_to_string(&disabled).unwrap();
    assert!(has_disable_comment(&source, 5, RULE_ID));

    let tsconfig = TsConfig {
        dir: root.clone(),
        paths: vec![],
        paths_dir: root.clone(),
        base_url: None,
    };
    let resolver = ImportResolver::new(&tsconfig);
    let graph = DepGraph::from_raw_maps(root.clone(), Default::default(), Default::default());
    let mocks = HashSet::new();
    let mut dependency_cache = HashMap::new();
    let mut findings = Vec::new();
    let mut context = DynamicCheckContext {
        root: &root,
        file: &disabled,
        resolver: &resolver,
        graph: &graph,
        mocks: &mocks,
        dependency_cache: &mut dependency_cache,
        findings: &mut findings,
    };
    check_dynamic_import(
        &mut context,
        ast::DynamicImport {
            specifier: Some("./missing.mts".to_string()),
            line: 1,
        },
    );
    assert_eq!(findings[0].import.as_deref(), Some("./missing.mts"));
    assert!(check(&root, &config, None)
        .unwrap()
        .iter()
        .all(|f| !f.file.contains("next-line-disabled")));
}

#[test]
fn mocked_dynamic_import_target_skips_transitive_dependency_checks() {
    let root = fixture();
    let tsconfig = load_tsconfig(&root.join("tsconfig.json")).unwrap();
    let resolver = ImportResolver::new(&tsconfig);
    let graph = DepGraph::build_with_plan(&root, &tsconfig, GraphBuildPlan::all()).unwrap();
    let test_file = root.join("tests").join("good.test.mts");
    let target = root.join("src").join("lazy.mts");
    let mut mocks = HashSet::new();
    mocks.insert(target);
    let mut dependency_cache = HashMap::new();
    let mut findings = Vec::new();
    let mut context = DynamicCheckContext {
        root: &root,
        file: &test_file,
        resolver: &resolver,
        graph: &graph,
        mocks: &mocks,
        dependency_cache: &mut dependency_cache,
        findings: &mut findings,
    };
    check_dynamic_import(
        &mut context,
        ast::DynamicImport {
            specifier: Some("../src/lazy.mts".to_string()),
            line: 1,
        },
    );
    assert!(findings.is_empty());
}

#[test]
fn repeated_dynamic_import_target_uses_dependency_cache() {
    let root = fixture();
    let tsconfig = load_tsconfig(&root.join("tsconfig.json")).unwrap();
    let resolver = ImportResolver::new(&tsconfig);
    let graph = DepGraph::build_with_plan(&root, &tsconfig, GraphBuildPlan::all()).unwrap();
    let test_file = root.join("tests").join("bad.test.mts");
    let mocks = HashSet::new();
    let mut dependency_cache = HashMap::new();
    let mut findings = Vec::new();
    let mut context = DynamicCheckContext {
        root: &root,
        file: &test_file,
        resolver: &resolver,
        graph: &graph,
        mocks: &mocks,
        dependency_cache: &mut dependency_cache,
        findings: &mut findings,
    };
    check_dynamic_import(
        &mut context,
        ast::DynamicImport {
            specifier: Some("../src/lazy.mts".to_string()),
            line: 1,
        },
    );
    let cache_len = context.dependency_cache.len();
    check_dynamic_import(
        &mut context,
        ast::DynamicImport {
            specifier: Some("../src/lazy.mts".to_string()),
            line: 1,
        },
    );
    assert_eq!(context.dependency_cache.len(), cache_len);
    assert!(!context.findings.is_empty());
}

#[test]
fn resolve_tsconfig_covers_explicit_and_default_paths() {
    let root = fixture();
    assert!(resolve_tsconfig(&root, Some(&root.join("tsconfig.json")))
        .unwrap()
        .base_url
        .is_some());
    let temp = tempfile::tempdir().unwrap();
    assert!(resolve_tsconfig(temp.path(), None)
        .unwrap()
        .base_url
        .is_none());
}

#[test]
fn resolve_mock_specifiers_keeps_unresolved_specifier_keys() {
    let root = fixture();
    let tsconfig = TsConfig {
        dir: root.clone(),
        paths: vec![],
        paths_dir: root.clone(),
        base_url: None,
    };
    let resolver = ImportResolver::new(&tsconfig);
    let mocks =
        resolve_mock_specifiers(&["external".to_string()], &root.join("test.mts"), &resolver);
    assert!(mocks.contains(&PathBuf::from("external")));
}
