use anyhow::Result;
use jsonc_parser::ParseOptions;
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};

const CONFIG_EXTENSIONS: &[&str] = &["yaml", "yml", "json", "jsonc"];

pub fn load_config<T: DeserializeOwned + Default>(
    root: &Path,
    cli_config: Option<&Path>,
    stems: &[&str],
) -> Result<T> {
    let Some(path) = find_config_path(root, cli_config, stems)? else {
        return Ok(T::default());
    };

    let source = std::fs::read_to_string(&path)?;
    parse_config(&source, &path)
}

fn find_config_path(
    root: &Path,
    cli_config: Option<&Path>,
    stems: &[&str],
) -> Result<Option<PathBuf>> {
    if let Some(path) = cli_config {
        let config_path = resolve(root, path);
        if !config_path.exists() {
            anyhow::bail!("config file does not exist: {}", config_path.display());
        }
        return Ok(Some(config_path));
    }

    let mut found_configs = Vec::new();
    for stem in stems {
        for extension in CONFIG_EXTENSIONS {
            let path = root.join(format!("{stem}.{extension}"));
            if path.exists() {
                found_configs.push(path);
            }
        }
        if !found_configs.is_empty() {
            break;
        }
    }

    match found_configs.len() {
        0 => Ok(None),
        1 => Ok(found_configs.pop()),
        _ => {
            let files = found_configs
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::bail!("multiple config files found under --root: {files}");
        }
    }
}

pub fn parse_config<T: DeserializeOwned>(source: &str, path: &Path) -> Result<T> {
    let extension = path.extension().and_then(|e| e.to_str());
    match extension {
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

pub fn resolve(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[derive(Default, serde::Deserialize)]
    struct TestConfig {
        name: String,
    }

    #[test]
    fn test_load_config_yaml() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("test.yaml");
        fs::write(&config_path, "name: hello\n").unwrap();

        let config: TestConfig = load_config(dir.path(), None, &["test"]).unwrap();
        assert_eq!(config.name, "hello");
    }

    #[test]
    fn test_load_config_json() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("test.json");
        fs::write(&config_path, "{\"name\": \"world\"}").unwrap();

        let config: TestConfig = load_config(dir.path(), None, &["test"]).unwrap();
        assert_eq!(config.name, "world");
    }

    #[test]
    fn test_load_config_priority() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("first.yaml"), "name: first\n").unwrap();
        fs::write(dir.path().join("second.yaml"), "name: second\n").unwrap();

        let config: TestConfig = load_config(dir.path(), None, &["first", "second"]).unwrap();
        assert_eq!(config.name, "first");
    }

    #[test]
    fn test_load_config_missing_returns_default() {
        let dir = tempdir().unwrap();
        let config: TestConfig = load_config(dir.path(), None, &["missing"]).unwrap();
        assert_eq!(config.name, "");
    }

    #[test]
    fn test_load_config_explicit() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("custom.yaml");
        fs::write(&config_path, "name: custom\n").unwrap();

        let config: TestConfig =
            load_config(dir.path(), Some(Path::new("custom.yaml")), &["test"]).unwrap();
        assert_eq!(config.name, "custom");
    }

    #[test]
    fn test_load_config_multiple_error() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("test.yaml"), "name: a\n").unwrap();
        fs::write(dir.path().join("test.json"), "{\"name\": \"b\"}").unwrap();

        let err = load_config::<TestConfig>(dir.path(), None, &["test"])
            .err()
            .unwrap();
        assert!(err.to_string().contains("multiple config files found"));
    }

    #[test]
    fn test_parse_config_jsonc() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("test.jsonc");
        fs::write(&config_path, "{\n  // comment\n  \"name\": \"jsonc\"\n}").unwrap();

        let config: TestConfig = load_config(dir.path(), None, &["test"]).unwrap();
        assert_eq!(config.name, "jsonc");
    }

    #[test]
    fn test_resolve_absolute() {
        let root = Path::new("/root");
        let path = Path::new("/abs/path");
        assert_eq!(resolve(root, path), path.to_path_buf());
    }

    #[test]
    fn test_load_config_explicit_missing() {
        let dir = tempdir().unwrap();
        let err = load_config::<TestConfig>(dir.path(), Some(Path::new("missing.yaml")), &["test"])
            .err()
            .unwrap();
        assert!(err.to_string().contains("config file does not exist"));
    }

    #[test]
    fn test_load_config_read_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.yaml");
        fs::create_dir(&path).unwrap(); // Dir instead of file will cause read error

        let err = load_config::<TestConfig>(dir.path(), None, &["test"])
            .err()
            .unwrap();
        assert!(err.to_string().contains("directory") || err.to_string().contains("failed"));

    }

    #[test]
    fn test_parse_config_jsonc_error() {
        let err = parse_config::<TestConfig>("{\n  \"name\": \n}", Path::new("test.jsonc"))
            .err()
            .unwrap();
        assert!(err.to_string().contains("Unexpected close brace"));

        let err = parse_config::<TestConfig>("", Path::new("test.jsonc"))
            .err()
            .unwrap();
        assert!(err.to_string().contains("invalid"));
    }

    #[test]
    fn test_parse_config_unsupported_extension() {
        let err = parse_config::<TestConfig>("", Path::new("test.toml"))
            .err()
            .unwrap();
        assert!(err
            .to_string()
            .contains("unsupported config file extension"));
    }

    #[test]
    fn test_parse_config_no_extension() {
        let err = parse_config::<TestConfig>("", Path::new("test"))
            .err()
            .unwrap();
        assert!(err
            .to_string()
            .contains("unsupported config file without extension"));
    }
}
