use anyhow::Result;
use jsonc_parser::ParseOptions;
use serde::Deserialize;
use std::path::{Path, PathBuf};

const CONFIG_FILE_STEM: &str = ".playwright-ast-coverage";
const CONFIG_EXTENSIONS: &[&str] = &["yaml", "yml", "json", "jsonc"];
const DEFAULT_FRONTEND_ROOT: &str = "app";
const DEFAULT_SELECTOR_ATTRIBUTES: &[&str] = &["data-testid", "data-pw"];
const PLAYWRIGHT_CONFIG_EXTENSIONS: &[&str] = &["ts", "mts", "cts", "js", "mjs", "cjs"];

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct FileConfig {
    frontend_root: Option<String>,
    playwright_config: Option<OneOrMany>,
    test_include: Vec<String>,
    test_exclude: Vec<String>,
    ignore_routes: Vec<String>,
    navigation_helpers: Vec<String>,
    selector_attributes: Option<Vec<String>>,
    selector_roots: Option<Vec<String>>,
    selector_include: Vec<String>,
    selector_exclude: Vec<String>,
}

#[derive(Deserialize)]
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
    let file_config = load_file_config(root, cli_config)?;
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

fn load_file_config(root: &Path, cli_config: Option<&Path>) -> Result<FileConfig> {
    let Some(config_path) = config_path(root, cli_config)? else {
        return Ok(FileConfig::default());
    };

    let source = std::fs::read_to_string(&config_path)?;
    parse_file_config(&source, &config_path)
}

fn config_path(root: &Path, cli_config: Option<&Path>) -> Result<Option<PathBuf>> {
    if let Some(path) = cli_config {
        let config_path = resolve(root, path);
        if !config_path.exists() {
            anyhow::bail!("config file does not exist: {}", config_path.display());
        }
        return Ok(Some(config_path));
    }

    let mut configs = Vec::new();
    for extension in CONFIG_EXTENSIONS {
        let path = root.join(format!("{CONFIG_FILE_STEM}.{extension}"));
        if path.exists() {
            configs.push(path);
        }
    }

    match configs.len() {
        0 => Ok(None),
        1 => Ok(configs.pop()),
        _ => {
            let files = configs
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::bail!("multiple config files found under --root: {files}");
        }
    }
}

fn parse_file_config(source: &str, config_path: &Path) -> Result<FileConfig> {
    match config_extension(config_path) {
        Some("yaml" | "yml") => Ok(serde_yaml::from_str(source)?),
        Some("json") => Ok(serde_json::from_str(source)?),
        Some("jsonc") => Ok(serde_json::from_value(jsonc_parser::parse_to_serde_value(
            source,
            &jsonc_parse_options(),
        )?)?),
        Some(extension) => anyhow::bail!(
            "unsupported config file extension .{extension}; supported extensions are .yaml, .yml, .json, and .jsonc"
        ),
        None => anyhow::bail!(
            "unsupported config file without extension; supported extensions are .yaml, .yml, .json, and .jsonc"
        ),
    }
}

fn config_extension(path: &Path) -> Option<&str> {
    path.extension().and_then(|extension| extension.to_str())
}

fn jsonc_parse_options() -> ParseOptions {
    ParseOptions {
        allow_comments: true,
        allow_loose_object_property_names: false,
        allow_trailing_commas: true,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
    }
}

fn resolve(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
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
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return false;
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
    fn explicit_unsupported_config_extension_errors() {
        let root = fixture_path(&["config", "unsupported-extension"]);
        let err = load_settings(
            &root,
            Some(Path::new(".playwright-ast-coverage.toml")),
            &[],
            None,
        )
        .err()
        .expect("expected unsupported config extension to fail");
        assert!(err
            .to_string()
            .contains("unsupported config file extension"));
    }

    #[test]
    fn explicit_extensionless_config_errors() {
        let root = fixture_path(&["config", "extensionless"]);
        let err = load_settings(
            &root,
            Some(Path::new(".playwright-ast-coverage")),
            &[],
            None,
        )
        .err()
        .expect("expected extensionless config to fail");
        assert!(err
            .to_string()
            .contains("unsupported config file without extension"));
    }

    #[test]
    fn reads_yaml_and_finds_default_playwright_config() {
        let root = fixture_path(&["config", "full"]);
        let settings = load_settings(&root, None, &[], None).unwrap();
        assert_eq!(settings.frontend_root, "web/app");
        assert_eq!(settings.test_exclude, vec!["**/skip/**"]);
        assert_eq!(settings.navigation_helpers, vec!["navigateTo"]);
        assert_eq!(settings.selector_roots, vec!["web/components"]);
        assert_eq!(settings.selector_include, vec!["web/components/**/*.tsx"]);
        assert_eq!(settings.selector_exclude, vec!["**/*.test.tsx"]);
        assert_eq!(
            settings.playwright_configs,
            vec![root.join("playwright.config.mts")]
        );
    }

    #[test]
    fn reads_yml_default_config() {
        let root = fixture_path(&["config", "yml"]);
        let settings = load_settings(&root, None, &[], None).unwrap();
        assert_eq!(settings.frontend_root, "web/yml-app");
    }

    #[test]
    fn reads_json_default_config() {
        let root = fixture_path(&["config", "json"]);
        let settings = load_settings(&root, None, &[], None).unwrap();
        assert_eq!(settings.frontend_root, "web/json-app");
        assert_eq!(settings.navigation_helpers, vec!["openJsonPath"]);
    }

    #[test]
    fn reads_jsonc_default_config() {
        let root = fixture_path(&["config", "jsonc"]);
        let settings = load_settings(&root, None, &[], None).unwrap();
        assert_eq!(settings.frontend_root, "web/jsonc-app");
        assert_eq!(settings.test_exclude, vec!["**/jsonc-skip/**"]);
    }

    #[test]
    fn jsonc_config_rejects_json5_object_keys() {
        let root = fixture_path(&["config", "invalid-jsonc"]);
        let err = load_settings(&root, None, &[], None)
            .err()
            .expect("expected invalid JSONC to fail");
        assert!(err
            .to_string()
            .contains("Expected string for object property"));
    }

    #[test]
    fn duplicate_default_config_files_error() {
        let root = fixture_path(&["config", "duplicate-defaults"]);
        let err = load_settings(&root, None, &[], None)
            .err()
            .expect("expected duplicate default configs to fail");
        assert!(err.to_string().contains("multiple config files found"));
        assert!(err.to_string().contains(".playwright-ast-coverage.yaml"));
        assert!(err.to_string().contains(".playwright-ast-coverage.json"));
    }

    #[test]
    fn yaml_playwright_config_path_is_resolved() {
        let root = fixture_path(&["config", "yaml-playwright-config"]);
        let settings = load_settings(&root, None, &[], None).unwrap();
        assert_eq!(
            settings.playwright_configs,
            vec![root.join("configs/playwright.config.ts")]
        );
    }

    #[test]
    fn cli_playwright_config_absolute_path_is_preserved() {
        let root = fixture_path(&["config", "missing-default"]);
        let config = root.join("custom.config.ts");
        let settings = load_settings(&root, None, std::slice::from_ref(&config), None).unwrap();
        assert_eq!(settings.playwright_configs, vec![config]);
    }

    #[test]
    fn invalid_yaml_errors() {
        let root = fixture_path(&["config", "invalid-yaml"]);
        let err = load_settings(&root, None, &[], None)
            .err()
            .expect("expected invalid YAML to fail");
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn selector_attributes_can_be_custom_or_disabled() {
        let custom = fixture_path(&["config", "selector-attributes-custom"]);
        let settings = load_settings(&custom, None, &[], None).unwrap();
        assert_eq!(
            settings.selector_attributes,
            vec!["data-test", "data-test-id"]
        );

        let disabled = fixture_path(&["config", "selector-attributes-disabled"]);
        let settings = load_settings(&disabled, None, &[], None).unwrap();
        assert!(settings.selector_attributes.is_empty());
    }

    #[test]
    fn old_default_config_name_is_ignored() {
        let root = fixture_path(&["config", "old-name"]);
        let settings = load_settings(&root, None, &[], None).unwrap();
        assert_eq!(settings.frontend_root, "app");
    }

    #[test]
    fn default_discovery_finds_all_root_playwright_configs() {
        let root = fixture_path(&["config", "multi-playwright-config"]);
        let settings = load_settings(&root, None, &[], None).unwrap();
        assert_eq!(
            settings.playwright_configs,
            vec![
                root.join("playwright.config.mts"),
                root.join("playwright.storybook.config.mts"),
            ]
        );
    }

    #[test]
    fn yaml_playwright_config_array_paths_are_resolved() {
        let root = fixture_path(&["config", "yaml-playwright-config-array"]);
        let settings = load_settings(&root, None, &[], None).unwrap();
        assert_eq!(
            settings.playwright_configs,
            vec![
                root.join("playwright.config.mts"),
                root.join("playwright.storybook.config.mts"),
            ]
        );
    }

    #[test]
    fn playwright_config_name_filter_rejects_paths_without_file_or_extension() {
        assert!(!is_playwright_config_name(Path::new("")));
        assert!(!is_playwright_config_name(Path::new("playwright")));
        assert!(!is_playwright_config_name(Path::new("playwright.config")));
    }
}
