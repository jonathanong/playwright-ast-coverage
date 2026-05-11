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
    selector_attributes: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct Settings {
    pub frontend_root: String,
    pub playwright_config: Option<PathBuf>,
    pub test_include: Vec<String>,
    pub test_exclude: Vec<String>,
    pub ignore_routes: Vec<String>,
    pub selector_attributes: Vec<String>,
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

    Ok(Settings {
        frontend_root: file_config
            .frontend_root
            .unwrap_or_else(|| DEFAULT_FRONTEND_ROOT.to_string()),
        playwright_config,
        test_include: file_config.test_include,
        test_exclude: file_config.test_exclude,
        ignore_routes: file_config.ignore_routes,
        selector_attributes: file_config
            .selector_attributes
            .unwrap_or_else(default_selector_attributes),
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
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn missing_default_config_uses_defaults() {
        let dir = TempDir::new().unwrap();
        let settings = load_settings(dir.path(), None, None).unwrap();
        assert_eq!(settings.frontend_root, "app");
        assert!(settings.playwright_config.is_none());
        assert_eq!(settings.selector_attributes, vec!["data-testid", "data-pw"]);
    }

    #[test]
    fn explicit_missing_config_errors() {
        let dir = TempDir::new().unwrap();
        let err = load_settings(dir.path(), Some(Path::new("missing.yaml")), None)
            .err()
            .expect("expected missing config to fail");
        assert!(err.to_string().contains("config file does not exist"));
    }

    #[test]
    fn reads_yaml_and_finds_default_playwright_config() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".playwright-ast-coverage.yaml"),
            "frontendRoot: web/app\ntestExclude: ['**/skip/**']\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("playwright.config.mts"),
            "export default {}",
        )
        .unwrap();

        let settings = load_settings(dir.path(), None, None).unwrap();
        assert_eq!(settings.frontend_root, "web/app");
        assert_eq!(settings.test_exclude, vec!["**/skip/**"]);
        assert_eq!(
            settings.playwright_config,
            Some(dir.path().join("playwright.config.mts"))
        );
    }

    #[test]
    fn yaml_playwright_config_path_is_resolved() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".playwright-ast-coverage.yaml"),
            "playwrightConfig: configs/playwright.config.ts\n",
        )
        .unwrap();
        let settings = load_settings(dir.path(), None, None).unwrap();
        assert_eq!(
            settings.playwright_config,
            Some(dir.path().join("configs/playwright.config.ts"))
        );
    }

    #[test]
    fn cli_playwright_config_absolute_path_is_preserved() {
        let dir = TempDir::new().unwrap();
        let config = dir.path().join("custom.config.ts");
        let settings = load_settings(dir.path(), None, Some(&config)).unwrap();
        assert_eq!(settings.playwright_config, Some(config));
    }

    #[test]
    fn invalid_yaml_errors() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".playwright-ast-coverage.yaml"),
            "frontendRoot: [",
        )
        .unwrap();
        let err = load_settings(dir.path(), None, None)
            .err()
            .expect("expected invalid YAML to fail");
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn selector_attributes_can_be_custom_or_disabled() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".playwright-ast-coverage.yaml"),
            "selectorAttributes: ['data-test', 'data-test-id']\n",
        )
        .unwrap();
        let settings = load_settings(dir.path(), None, None).unwrap();
        assert_eq!(
            settings.selector_attributes,
            vec!["data-test", "data-test-id"]
        );

        fs::write(
            dir.path().join(".playwright-ast-coverage.yaml"),
            "selectorAttributes: []\n",
        )
        .unwrap();
        let settings = load_settings(dir.path(), None, None).unwrap();
        assert!(settings.selector_attributes.is_empty());
    }

    #[test]
    fn old_default_config_name_is_ignored() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".playwright-path-coverage.yaml"),
            "frontendRoot: ignored/app\n",
        )
        .unwrap();
        let settings = load_settings(dir.path(), None, None).unwrap();
        assert_eq!(settings.frontend_root, "app");
    }
}
