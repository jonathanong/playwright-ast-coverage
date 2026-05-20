use super::*;
use std::path::PathBuf;

#[test]
fn rule_enabled_defaults_true_and_reads_false() {
    let config = Config::from_yaml(
        r#"
rules:
  default-rule: {}
  disabled-rule:
    enabled: false
"#,
    )
    .unwrap();

    assert!(config.is_rule_enabled("missing-rule"));
    assert!(config.is_rule_enabled("default-rule"));
    assert!(!config.is_rule_enabled("disabled-rule"));
}

#[test]
fn augment_from_gitignore_adds_plain_directory_names_once() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/ast-snippets/config/gitignore-project");
    let mut config = Config {
        filesystem: FilesystemConfig {
            skip_directories: vec!["dist".to_string()],
            skip_file_patterns: vec![],
        },
        projects: HashMap::new(),
        rules: HashMap::new(),
    };

    config.augment_from_gitignore(&root);

    assert_eq!(
        config.filesystem.skip_directories,
        vec!["dist".to_string(), "node_modules".to_string()]
    );
}

#[test]
fn augment_from_gitignore_ignores_missing_file() {
    let mut config = Config::default();

    config.augment_from_gitignore(Path::new("/no/such/project"));

    assert!(config.filesystem.skip_directories.is_empty());
}

#[test]
fn load_codebase_config_uses_explicit_config_path() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/config-v2/disabled-rule");
    let config_path = root.join(".no-mistakes.yml");

    let config = load_codebase_config_with_path(&root, Some(&config_path)).unwrap();

    assert!(config.is_rule_enabled("active-rule"));
    assert!(!config.is_rule_enabled("disabled-rule"));
}

#[test]
fn load_codebase_config_defaults_when_no_config_exists() {
    let root = tempfile::tempdir().unwrap();

    let config = load_codebase_config_with_path(root.path(), None).unwrap();

    assert!(config.filesystem.skip_directories.is_empty());
    assert!(config.projects.is_empty());
    assert!(config.rules.is_empty());
}

#[test]
fn load_config_with_explicit_config_uses_config_parent_gitignore() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/config-v2/explicit-config-parent");
    let nested = root.join("nested");

    let config = load_config_with_path(&nested, Some(Path::new("../.no-mistakes.yml"))).unwrap();

    assert_eq!(config.filesystem.skip_directories, vec!["from-config"]);
}

#[test]
fn load_codebase_config_finds_parent_guardrails_config() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis/codebase-intel");
    let nested = root.join("packages/api/src");

    let config = load_codebase_config_with_path(&nested, None).unwrap();
    let routes: RouteOptions = config.rule_options("route-consistency");

    assert_eq!(routes.backend_pattern, "packages/api/src/**/*.mts");
    assert_eq!(routes.frontend_root, "packages/web/app");
}

#[test]
fn load_codebase_config_finds_parent_no_mistakes_config() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis/unique-exports-config-disabled");
    let nested = root.join("src/nested");

    let config = load_codebase_config_with_path(&nested, None).unwrap();

    assert!(!config.is_rule_enabled("unique-exports"));
}

#[test]
fn load_codebase_config_rejects_duplicate_parent_configs() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/config-v2/duplicate-stems");

    let error = load_codebase_config_with_path(&root, None).unwrap_err();

    assert!(error.to_string().contains("multiple config files found"));
}

#[test]
fn project_roots_for_rule_covers_default_and_unmatched_projects() {
    let root = Path::new("/repo");
    let config = Config::from_yaml(
        r#"
projects:
  app:
    rules: [unique-exports]
  other:
    rules: [different-rule]
"#,
    )
    .unwrap();

    assert_eq!(
        config.project_roots_for_rule(root, "unique-exports"),
        vec![PathBuf::from("/repo")]
    );
    assert!(config
        .project_roots_for_rule(root, "missing-rule")
        .is_empty());
}
