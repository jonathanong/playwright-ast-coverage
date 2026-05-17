use super::*;
use crate::config::v2::schema::Project;

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
fn project_include_restricts_default_test_globs() {
    let mut config = NoMistakesConfig::default();
    config.projects.insert(
        "storybook".to_string(),
        Project {
            root: Some("web/storybook".to_string()),
            include: vec!["**/*.test.tsx".to_string()],
            rules: vec![super::super::RULE_ID.to_string()],
            ..Default::default()
        },
    );
    config.projects.insert(
        "root-tests".to_string(),
        Project {
            include: vec!["tests/**/*.test.ts".to_string()],
            rules: vec![super::super::RULE_ID.to_string()],
            ..Default::default()
        },
    );
    let filter = test_filter(Path::new("."), &config).unwrap();
    assert!(filter.is_match("web/storybook/__tests__/a.test.tsx".to_string()));
    assert!(filter.is_match("tests/a.test.ts".to_string()));
    assert!(!filter.is_match("web/components/a.test.tsx".to_string()));
}

#[test]
fn scoped_glob_leaves_root_project_includes_unprefixed() {
    assert_eq!(scoped_glob(".", "tests/**/*.test.ts"), "tests/**/*.test.ts");
    assert_eq!(
        scoped_glob("web/storybook", "./**/*.test.tsx"),
        "web/storybook/**/*.test.tsx"
    );
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

#[test]
fn explicit_config_files_skip_default_discovery() {
    let root = crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/test-no-unmocked-dynamic-imports"),
    );
    let mut config = NoMistakesConfig::default();
    config.tests.vitest.configs = Some(crate::config::v2::schema::StringOrList::One(
        "jest.config.mjs".to_string(),
    ));
    let files = config_files(&root, &config);
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("jest.config.mjs"));
}

#[test]
fn default_config_discovery_normalizes_existing_files() {
    let root = crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/test-no-unmocked-dynamic-imports"),
    );
    let files = config_files(&root, &NoMistakesConfig::default());
    assert!(files.iter().any(|file| file.ends_with("vitest.config.mts")));
    assert!(files.iter().any(|file| file.ends_with("jest.config.mjs")));
}
