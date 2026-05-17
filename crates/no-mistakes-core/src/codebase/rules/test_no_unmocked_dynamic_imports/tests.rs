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
    let mut findings = Vec::new();
    check_dynamic_import(
        &root,
        &disabled,
        ast::DynamicImport {
            specifier: Some("./missing.mts".to_string()),
            line: 1,
        },
        &resolver,
        &graph,
        &Default::default(),
        &mut findings,
    );
    assert_eq!(findings[0].import.as_deref(), Some("./missing.mts"));
    assert!(check(&root, &config, None)
        .unwrap()
        .iter()
        .all(|f| !f.file.contains("next-line-disabled")));
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
