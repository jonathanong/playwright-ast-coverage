use anyhow::Result;
use no_mistakes_core::config::{self, resolve};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const CONFIG_STEMS: &[&str] = &[".no-mistakes", ".playwright-ast-coverage"];
const DEFAULT_FRONTEND_ROOT: &str = "app";
const DEFAULT_SELECTOR_ATTRIBUTES: &[&str] = &["data-testid", "data-pw"];
const PLAYWRIGHT_CONFIG_EXTENSIONS: &[&str] = &["ts", "mts", "cts", "js", "mjs", "cjs"];

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RootConfig {
    #[serde(flatten)]
    legacy: FileConfig,
    playwright_ast_coverage: Option<FileConfig>,
}

#[derive(Default, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
struct FileConfig {
    frontend_root: Option<String>,
    playwright_config: Option<OneOrMany>,
    test_include: Vec<String>,
    test_exclude: Vec<String>,
    ignore_routes: Vec<String>,
    navigation_helpers: Vec<String>,
    selector_attributes: Option<Vec<String>>,
    component_selector_attributes: BTreeMap<String, String>,
    html_ids: bool,
    selector_roots: Option<Vec<String>>,
    selector_include: Vec<String>,
    selector_exclude: Vec<String>,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

#[derive(Clone)]
pub struct Settings {
    pub frontend_root: String,
    pub playwright_configs: Vec<PathBuf>,
    pub project: Option<String>,
    pub test_include: Vec<String>,
    pub test_exclude: Vec<String>,
    pub ignore_routes: Vec<String>,
    pub navigation_helpers: Vec<String>,
    pub selector_attributes: Vec<String>,
    pub component_selector_attributes: BTreeMap<String, String>,
    pub html_ids: bool,
    pub selector_roots: Vec<String>,
    pub selector_include: Vec<String>,
    pub selector_exclude: Vec<String>,
}

pub fn load_settings(
    root: &Path,
    cli_config: Option<&Path>,
    cli_playwright_configs: &[PathBuf],
    cli_project: Option<String>,
) -> Result<Settings> {
    let root_config: RootConfig = config::load_config(root, cli_config, CONFIG_STEMS)?;
    let file_config = root_config
        .playwright_ast_coverage
        .unwrap_or(root_config.legacy);

    let playwright_configs = if !cli_playwright_configs.is_empty() {
        cli_playwright_configs
            .iter()
            .map(|path| resolve(root, path))
            .collect()
    } else if let Some(paths) = file_config.playwright_config.as_ref() {
        paths
            .values()
            .iter()
            .map(|path| resolve(root, Path::new(path)))
            .collect()
    } else {
        find_default_playwright_configs(root)?
    };

    let frontend_root = file_config
        .frontend_root
        .unwrap_or_else(|| DEFAULT_FRONTEND_ROOT.to_string());
    let selector_roots = file_config
        .selector_roots
        .unwrap_or_else(|| vec![frontend_root.clone()]);

    Ok(Settings {
        frontend_root,
        playwright_configs,
        project: cli_project,
        test_include: file_config.test_include,
        test_exclude: file_config.test_exclude,
        ignore_routes: file_config.ignore_routes,
        navigation_helpers: file_config.navigation_helpers,
        selector_attributes: file_config
            .selector_attributes
            .unwrap_or_else(default_selector_attributes),
        component_selector_attributes: file_config.component_selector_attributes,
        html_ids: file_config.html_ids,
        selector_roots,
        selector_include: file_config.selector_include,
        selector_exclude: file_config.selector_exclude,
    })
}

impl OneOrMany {
    fn values(&self) -> Vec<String> {
        match self {
            OneOrMany::One(value) => vec![value.clone()],
            OneOrMany::Many(values) => values.clone(),
        }
    }
}

fn find_default_playwright_configs(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut configs = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || !is_playwright_config_name(&path) {
            continue;
        }
        configs.push(path);
    }
    configs.sort();
    Ok(configs)
}

fn is_playwright_config_name(path: &Path) -> bool {
    let name = match path.file_name().and_then(|name| name.to_str()) {
        Some(name) => name,
        None => return false,
    };
    let extension = match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension) => extension,
        None => return false,
    };

    name.starts_with("playwright")
        && name.contains(".config.")
        && PLAYWRIGHT_CONFIG_EXTENSIONS.contains(&extension)
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
        let settings = load_settings(&root, None, &[], None).unwrap();
        assert_eq!(settings.frontend_root, "app");
        assert!(settings.playwright_configs.is_empty());
        assert_eq!(settings.selector_attributes, vec!["data-testid", "data-pw"]);
        assert!(settings.component_selector_attributes.is_empty());
        assert!(!settings.html_ids);
        assert_eq!(settings.selector_roots, vec!["app"]);
    }

    #[test]
    fn explicit_missing_config_errors() {
        let root = fixture_path(&["config", "missing-default"]);
        let err = load_settings(&root, Some(Path::new("missing.yaml")), &[], None)
            .err()
            .expect("expected missing config to fail");
        assert!(err.to_string().contains("config file does not exist"));
    }

    #[test]
    fn reads_yaml_and_finds_default_playwright_config() {
        let root = fixture_path(&["config", "full"]);
        let settings = load_settings(&root, None, &[], None).unwrap();
        assert_eq!(settings.frontend_root, "web/app");
        assert_eq!(settings.test_exclude, vec!["**/skip/**"]);
        assert_eq!(settings.navigation_helpers, vec!["navigateTo"]);
        assert!(settings.html_ids);
        assert_eq!(settings.selector_roots, vec!["web/components"]);
        assert_eq!(settings.selector_include, vec!["web/components/**/*.tsx"]);
        assert_eq!(settings.selector_exclude, vec!["**/*.test.tsx"]);
        assert_eq!(
            settings.playwright_configs,
            vec![root.join("playwright.config.mts")]
        );
    }

    #[test]
    fn no_mistakes_config_has_priority_and_supports_nesting() {
        let root = fixture_path(&["config", "no-mistakes-priority"]);
        let settings = load_settings(&root, None, &[], None).unwrap();
        assert_eq!(settings.frontend_root, "no-mistakes-app");

        let root = fixture_path(&["config", "no-mistakes-nested"]);
        let settings = load_settings(&root, None, &[], None).unwrap();
        assert_eq!(settings.frontend_root, "nested-app");
    }

    #[test]
    fn test_one_or_many_values() {
        let one = OneOrMany::One("a".to_string());
        assert_eq!(one.values(), vec!["a"]);
        let many = OneOrMany::Many(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(many.values(), vec!["a", "b"]);
    }

    #[test]
    fn test_is_playwright_config_name_edge_cases() {
        assert!(!is_playwright_config_name(Path::new("")));
        assert!(!is_playwright_config_name(Path::new(
            "playwright.config.txt"
        )));
        assert!(!is_playwright_config_name(Path::new(
            "notplaywright.config.ts"
        )));
        assert!(!is_playwright_config_name(Path::new("playwright.config")));
        assert!(!is_playwright_config_name(Path::new("playwrightconfig")));
    }

    #[test]
    fn test_playwright_config_from_file() {
        let root = fixture_path(&["config", "playwright-config-array"]);
        let settings = load_settings(&root, None, &[], None).unwrap();
        assert_eq!(settings.playwright_configs.len(), 2);
        assert!(settings.playwright_configs[0].ends_with("playwright.config.ts"));
        assert!(settings.playwright_configs[1].ends_with("playwright.other.config.ts"));

        let root = fixture_path(&["config", "playwright-config-single"]);
        let settings = load_settings(&root, None, &[], None).unwrap();
        assert_eq!(settings.playwright_configs.len(), 1);
        assert!(settings.playwright_configs[0].ends_with("playwright.config.ts"));
    }
}
