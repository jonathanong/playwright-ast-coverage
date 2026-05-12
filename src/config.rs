use anyhow::Result;
use serde::Deserialize;
use std::path::{Path, PathBuf};

const DEFAULT_CONFIG_FILE: &str = ".playwright-ast-coverage.yaml";
const DEFAULT_FRONTEND_ROOT: &str = "app";
const DEFAULT_SELECTOR_ATTRIBUTES: &[&str] = &["data-testid", "data-pw"];
const PLAYWRIGHT_CONFIG_NAMES: &[&str] = &[
    "playwright.config.ts",
    "playwright.config.mts",
    "playwright.config.cts",
    "playwright.config.js",
    "playwright.config.mjs",
    "playwright.config.cjs",
];

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct FileConfig {
    frontend_root: Option<String>,
    playwright_config: Option<String>,
    test_include: Vec<String>,
    test_exclude: Vec<String>,
    ignore_routes: Vec<String>,
    navigation_helpers: Vec<String>,
    selector_attributes: Option<Vec<String>>,
    selector_roots: Option<Vec<String>>,
    selector_include: Vec<String>,
    selector_exclude: Vec<String>,
}

#[derive(Clone)]
pub struct Settings {
    pub frontend_root: String,
    pub playwright_config: Option<PathBuf>,
    pub test_include: Vec<String>,
    pub test_exclude: Vec<String>,
    pub ignore_routes: Vec<String>,
    pub navigation_helpers: Vec<String>,
    pub selector_attributes: Vec<String>,
    pub selector_roots: Vec<String>,
    pub selector_include: Vec<String>,
    pub selector_exclude: Vec<String>,
}

pub fn load_settings(
    root: &Path,
    cli_config: Option<&Path>,
    cli_playwright_config: Option<&Path>,
) -> Result<Settings> {
    let file_config = load_file_config(root, cli_config)?;
    let playwright_config = cli_playwright_config
        .map(|path| resolve(root, path))
        .or_else(|| {
            file_config
                .playwright_config
                .as_deref()
                .map(|path| resolve(root, Path::new(path)))
        })
        .or_else(|| find_default_playwright_config(root));

    let frontend_root = file_config
        .frontend_root
        .unwrap_or_else(|| DEFAULT_FRONTEND_ROOT.to_string());
    let selector_roots = file_config
        .selector_roots
        .unwrap_or_else(|| vec![frontend_root.clone()]);

    Ok(Settings {
        frontend_root,
        playwright_config,
        test_include: file_config.test_include,
        test_exclude: file_config.test_exclude,
        ignore_routes: file_config.ignore_routes,
        navigation_helpers: file_config.navigation_helpers,
        selector_attributes: file_config
            .selector_attributes
            .unwrap_or_else(default_selector_attributes),
        selector_roots,
        selector_include: file_config.selector_include,
        selector_exclude: file_config.selector_exclude,
    })
}

fn load_file_config(root: &Path, cli_config: Option<&Path>) -> Result<FileConfig> {
    let config_path = cli_config
        .map(|path| resolve(root, path))
        .unwrap_or_else(|| root.join(DEFAULT_CONFIG_FILE));

    if !config_path.exists() {
        if cli_config.is_some() {
            anyhow::bail!("config file does not exist: {}", config_path.display());
        }
        return Ok(FileConfig::default());
    }

    let source = std::fs::read_to_string(&config_path)?;
    Ok(serde_yaml::from_str(&source)?)
}

fn resolve(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn find_default_playwright_config(root: &Path) -> Option<PathBuf> {
    PLAYWRIGHT_CONFIG_NAMES
        .iter()
        .map(|name| root.join(name))
        .find(|path| path.exists())
}

fn default_selector_attributes() -> Vec<String> {
    DEFAULT_SELECTOR_ATTRIBUTES
        .iter()
        .map(|attribute| attribute.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::fixture_path;

    #[test]
    fn missing_default_config_uses_defaults() {
        let root = fixture_path(&["config", "missing-default"]);
        let settings = load_settings(&root, None, None).unwrap();
        assert_eq!(settings.frontend_root, "app");
        assert!(settings.playwright_config.is_none());
        assert_eq!(settings.selector_attributes, vec!["data-testid", "data-pw"]);
        assert_eq!(settings.selector_roots, vec!["app"]);
    }

    #[test]
    fn explicit_missing_config_errors() {
        let root = fixture_path(&["config", "missing-default"]);
        let err = load_settings(&root, Some(Path::new("missing.yaml")), None)
            .err()
            .expect("expected missing config to fail");
        assert!(err.to_string().contains("config file does not exist"));
    }

    #[test]
    fn reads_yaml_and_finds_default_playwright_config() {
        let root = fixture_path(&["config", "full"]);
        let settings = load_settings(&root, None, None).unwrap();
        assert_eq!(settings.frontend_root, "web/app");
        assert_eq!(settings.test_exclude, vec!["**/skip/**"]);
        assert_eq!(settings.navigation_helpers, vec!["navigateTo"]);
        assert_eq!(settings.selector_roots, vec!["web/components"]);
        assert_eq!(settings.selector_include, vec!["web/components/**/*.tsx"]);
        assert_eq!(settings.selector_exclude, vec!["**/*.test.tsx"]);
        assert_eq!(
            settings.playwright_config,
            Some(root.join("playwright.config.mts"))
        );
    }

    #[test]
    fn yaml_playwright_config_path_is_resolved() {
        let root = fixture_path(&["config", "yaml-playwright-config"]);
        let settings = load_settings(&root, None, None).unwrap();
        assert_eq!(
            settings.playwright_config,
            Some(root.join("configs/playwright.config.ts"))
        );
    }

    #[test]
    fn cli_playwright_config_absolute_path_is_preserved() {
        let root = fixture_path(&["config", "missing-default"]);
        let config = root.join("custom.config.ts");
        let settings = load_settings(&root, None, Some(&config)).unwrap();
        assert_eq!(settings.playwright_config, Some(config));
    }

    #[test]
    fn invalid_yaml_errors() {
        let root = fixture_path(&["config", "invalid-yaml"]);
        let err = load_settings(&root, None, None)
            .err()
            .expect("expected invalid YAML to fail");
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn selector_attributes_can_be_custom_or_disabled() {
        let custom = fixture_path(&["config", "selector-attributes-custom"]);
        let settings = load_settings(&custom, None, None).unwrap();
        assert_eq!(
            settings.selector_attributes,
            vec!["data-test", "data-test-id"]
        );

        let disabled = fixture_path(&["config", "selector-attributes-disabled"]);
        let settings = load_settings(&disabled, None, None).unwrap();
        assert!(settings.selector_attributes.is_empty());
    }

    #[test]
    fn old_default_config_name_is_ignored() {
        let root = fixture_path(&["config", "old-name"]);
        let settings = load_settings(&root, None, None).unwrap();
        assert_eq!(settings.frontend_root, "app");
    }
}
