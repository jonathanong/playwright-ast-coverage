use anyhow::Result;
use std::path::{Path, PathBuf};

use super::legacy;
use super::schema::NoMistakesConfig;
use super::ToolKind;
use crate::config::{parse_config, resolve, CONFIG_EXTENSIONS};

const V2_STEMS: &[&str] = &[".no-mistakes"];
const TOOL_STEMS: &[(&str, ToolKind)] = &[
    (".playwright-ast-coverage", ToolKind::Playwright),
    (".react-traits", ToolKind::ReactTraits),
    (".next-to-fetch", ToolKind::NextToFetch),
];
const GUARDRAILS_STEM: &str = ".guardrailsrc";

/// Load the unified `.no-mistakes.yml` (or a recognized legacy config) from
/// `root`, returning a [`NoMistakesConfig`].
///
/// Discovery order:
/// 1. `cli_config` if provided.
/// 2. `.no-mistakes.{yaml,yml,json,jsonc}` in `root`.
/// 3. Per-tool legacy stems in `root` (`.playwright-ast-coverage.*`, etc.).
/// 4. `.guardrailsrc.*` walking upward from `root`.
/// 5. Empty default.
pub fn load_v2_config(root: &Path, cli_config: Option<&Path>) -> Result<NoMistakesConfig> {
    if let Some(path) = cli_config {
        let resolved = resolve(root, path);
        if !resolved.exists() {
            anyhow::bail!("config file does not exist: {}", resolved.display());
        }
        let source = std::fs::read_to_string(&resolved)?;
        return detect_and_parse(&source, &resolved);
    }

    if let Some((path, source)) = find_by_stems(root, V2_STEMS)? {
        return parse_config::<NoMistakesConfig>(&source, &path);
    }

    for (stem, kind) in TOOL_STEMS {
        if let Some((path, source)) = find_by_stems(root, &[stem])? {
            return legacy::from_tool_config(&source, &path, *kind);
        }
    }

    if let Some((path, source)) = find_guardrails(root)? {
        return legacy::from_guardrails_config(&source, &path);
    }

    Ok(NoMistakesConfig::default())
}

fn detect_and_parse(source: &str, path: &Path) -> Result<NoMistakesConfig> {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    match stem {
        ".playwright-ast-coverage" => legacy::from_tool_config(source, path, ToolKind::Playwright),
        ".react-traits" => legacy::from_tool_config(source, path, ToolKind::ReactTraits),
        ".next-to-fetch" => legacy::from_tool_config(source, path, ToolKind::NextToFetch),
        s if s == ".guardrailsrc" || s == "guardrailsrc" => {
            legacy::from_guardrails_config(source, path)
        }
        _ => parse_config::<NoMistakesConfig>(source, path),
    }
}

pub(super) fn find_by_stems(root: &Path, stems: &[&str]) -> Result<Option<(PathBuf, String)>> {
    let mut found = Vec::new();
    for stem in stems {
        for ext in CONFIG_EXTENSIONS {
            let path = root.join(format!("{stem}.{ext}"));
            if path.exists() {
                found.push(path);
            }
        }
        if !found.is_empty() {
            break;
        }
    }
    match found.len() {
        0 => Ok(None),
        1 => {
            let path = found.remove(0);
            let source = std::fs::read_to_string(&path)?;
            Ok(Some((path, source)))
        }
        _ => {
            let files = found
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::bail!("multiple config files found under --root: {files}");
        }
    }
}

fn find_guardrails(start: &Path) -> Result<Option<(PathBuf, String)>> {
    let mut current = start.to_path_buf();
    loop {
        for ext in CONFIG_EXTENSIONS {
            let path = current.join(format!("{GUARDRAILS_STEM}.{ext}"));
            if path.exists() {
                let source = std::fs::read_to_string(&path)?;
                return Ok(Some((path, source)));
            }
        }
        if !current.pop() {
            return Ok(None);
        }
    }
}
