use super::*;

#[test]
fn extracts_setup_files_and_include_exclude_strings() {
    let source = "export default { test: { include: ['a.test.mts'], exclude: ['b.test.mts'], setupFiles: ['./setup.mts'] } }";
    assert_eq!(
        extract_property_strings(source, "setupFiles"),
        vec!["./setup.mts"]
    );
    assert_eq!(
        extract_property_strings(source, "include"),
        vec!["a.test.mts"]
    );
    assert_eq!(
        extract_property_strings(source, "exclude"),
        vec!["b.test.mts"]
    );
}

#[test]
fn default_filter_matches_vitest_and_jest_test_files() {
    let config = NoMistakesConfig::default();
    let filter = test_filter(Path::new("."), &config).unwrap();
    assert!(filter.is_match("src/a.test.mts".to_string()));
    assert!(filter.is_match("src/a.spec.ts".to_string()));
    assert!(filter.is_match("src/__tests__/a.js".to_string()));
    assert!(!filter.is_match("src/a.mts".to_string()));
}

#[test]
fn setup_files_resolves_config_relative_existing_files() {
    let root = crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/test-no-unmocked-dynamic-imports"),
    );
    let config = crate::config::v2::load_v2_config(&root, None).unwrap();
    let files = setup_files(&root, &config).unwrap();
    assert!(files
        .iter()
        .any(|path| path.ends_with("tests/setup-vitest.mts")));
    assert!(files
        .iter()
        .any(|path| path.ends_with("tests/setup-jest.mts")));
}
