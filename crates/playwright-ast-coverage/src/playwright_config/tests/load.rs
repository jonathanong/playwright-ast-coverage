use crate::playwright_config::load::{load, load_many};
use crate::playwright_config::types::TestProject;
use crate::test_support::fixture_path;
use std::path::{Path, PathBuf};

#[test]
fn load_without_config_uses_default_project() {
    let parsed = load_many(Path::new("/repo"), &[], None).unwrap();
    assert_eq!(parsed.projects[0].test_dir, ".");
    assert!(parsed.projects[0]
        .test_match
        .contains(&"**/*.spec.ts".to_string()));
}

#[test]
fn load_many_without_configs_rejects_project_filter() {
    let err = load_many(Path::new("/repo"), &[], Some("storybook"))
        .err()
        .expect("expected project filter without config to fail");
    assert!(err.to_string().contains("--project requires"));
}

#[test]
fn load_missing_config_errors() {
    let err = load(Path::new("/repo"), Path::new("/repo/missing.ts"))
        .err()
        .expect("expected missing config to fail");
    assert!(err.to_string().contains("does not exist"));
}

#[test]
fn load_many_errors_when_project_filter_matches_no_config() {
    let dir = fixture_path(&["scan-config", "multi-playwright-config"]);
    let config = dir.join("playwright.config.mts");
    let err = load_many(&dir, &[config], Some("missing"))
        .err()
        .expect("expected missing config name to fail");
    assert!(err.to_string().contains("no Playwright config found"));
}

#[test]
fn load_many_errors_when_config_is_missing_name_for_filter() {
    let dir = fixture_path(&["ast-snippets", "playwright_config", "load-existing"]);
    let config = dir.join("playwright.config.ts");
    let err = load_many(&dir, &[config], Some("project"))
        .err()
        .expect("expected missing name to fail");
    assert!(err.to_string().contains("must define top-level name"));
}

#[test]
fn validate_config_names_errors_on_duplicate_names() {
    let dir = fixture_path(&["scan-config", "multi-playwright-config"]);
    let config = dir.join("playwright.config.mts");
    // Loading the same config twice should error with "duplicated" since both have same name
    let err = load_many(&dir, &[config.clone(), config], None)
        .err()
        .expect("expected duplicate config name to fail");
    assert!(err.to_string().contains("duplicated"));
}

#[test]
fn load_existing_config_reads_and_parses() {
    let dir = fixture_path(&["ast-snippets", "playwright_config", "load-existing"]);
    let config = dir.join("playwright.config.ts");
    let parsed = load(&dir, &config).unwrap();
    assert_eq!(parsed.projects[0].test_dir, "./tests");
}

#[test]
fn load_directory_config_path_returns_read_error() {
    let dir = fixture_path(&["ast-snippets", "playwright_config", "load-existing"]);
    let err = load(&dir, &dir)
        .err()
        .expect("expected directory config path to fail");
    assert!(!err.to_string().is_empty());
}

#[test]
fn test_dir_resolves_absolute_relative_and_relative_config_dir() {
    let absolute = TestProject {
        config_dir: PathBuf::from("/repo"),
        test_dir: "/tmp/tests".to_string(),
        test_match: vec![],
        test_ignore: vec![],
        base_url: None,
        test_id_attribute: "data-testid".to_string(),
    };
    assert_eq!(
        absolute.test_dir(Path::new("/repo")),
        PathBuf::from("/tmp/tests")
    );

    let absolute_config_relative_test_dir = TestProject {
        config_dir: PathBuf::from("/repo"),
        test_dir: "tests".to_string(),
        test_match: vec![],
        test_ignore: vec![],
        base_url: None,
        test_id_attribute: "data-testid".to_string(),
    };
    assert_eq!(
        absolute_config_relative_test_dir.test_dir(Path::new("/repo")),
        PathBuf::from("/repo/tests")
    );

    let relative_config = TestProject {
        config_dir: PathBuf::from("config"),
        test_dir: "tests".to_string(),
        test_match: vec![],
        test_ignore: vec![],
        base_url: None,
        test_id_attribute: "data-testid".to_string(),
    };
    assert_eq!(
        relative_config.test_dir(Path::new("/repo")),
        PathBuf::from("/repo/config/tests")
    );
}
