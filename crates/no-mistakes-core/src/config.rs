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
mod tests;
