use anyhow::Result;
use serde::Deserialize;
use std::path::{Path, PathBuf};

const EXTENSIONS: &[&str] = &["mts", "ts", "tsx", "mjs", "js", "jsx", "cjs", "cts"];

#[derive(Debug, Clone, Default)]
pub(crate) struct TsConfig {
    pub paths_dir: PathBuf,
    pub base_url: Option<PathBuf>,
    pub paths: Vec<(String, Vec<String>)>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawTsConfig {
    compiler_options: Option<CompilerOptions>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct CompilerOptions {
    base_url: Option<String>,
    paths: Option<std::collections::BTreeMap<String, Vec<String>>>,
}

pub(crate) fn load_tsconfig(root: &Path, explicit: Option<&Path>) -> Result<TsConfig> {
    let path = explicit
        .map(|path| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                root.join(path)
            }
        })
        .or_else(|| find_tsconfig(root));
    let Some(path) = path else {
        return Ok(TsConfig {
            paths_dir: root.to_path_buf(),
            base_url: None,
            paths: Vec::new(),
        });
    };
    let source = std::fs::read_to_string(&path)?;
    let raw: RawTsConfig = serde_json::from_value(jsonc_parser::parse_to_serde_value(
        &source,
        &jsonc_parser::ParseOptions::default(),
    )?)?;
    let dir = path.parent().unwrap_or(root).to_path_buf();
    let options = raw.compiler_options.unwrap_or_default();
    let base_url = options.base_url.map(|url| dir.join(url));
    let paths = options
        .paths
        .unwrap_or_default()
        .into_iter()
        .collect::<Vec<_>>();
    Ok(TsConfig {
        paths_dir: dir,
        base_url,
        paths,
    })
}

pub(crate) fn resolve_import(
    specifier: &str,
    current_file: &Path,
    root: &Path,
    tsconfig: &TsConfig,
) -> Option<PathBuf> {
    if specifier.starts_with('.') {
        return current_file
            .parent()
            .and_then(|parent| resolve_candidate(&parent.join(specifier)));
    }
    for (pattern, targets) in &tsconfig.paths {
        if let Some(capture) = match_pattern(pattern, specifier) {
            for target in targets {
                let replaced = target.replace('*', capture);
                let base = tsconfig
                    .base_url
                    .as_ref()
                    .unwrap_or(&tsconfig.paths_dir)
                    .join(replaced);
                if let Some(path) = resolve_candidate(&base) {
                    return Some(path);
                }
            }
        }
    }
    tsconfig
        .base_url
        .as_ref()
        .and_then(|base| resolve_candidate(&base.join(specifier)))
        .or_else(|| resolve_candidate(&root.join(specifier)))
}

fn find_tsconfig(root: &Path) -> Option<PathBuf> {
    let mut dir = Some(root);
    while let Some(path) = dir {
        let candidate = path.join("tsconfig.json");
        if candidate.exists() {
            return Some(candidate);
        }
        dir = path.parent();
    }
    None
}

fn match_pattern<'a>(pattern: &str, specifier: &'a str) -> Option<&'a str> {
    if let Some((prefix, suffix)) = pattern.split_once('*') {
        return specifier
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_suffix(suffix));
    }
    (pattern == specifier).then_some("")
}

fn resolve_candidate(path: &Path) -> Option<PathBuf> {
    if path.is_file() && is_source(path) {
        return path
            .canonicalize()
            .ok()
            .or_else(|| Some(path.to_path_buf()));
    }
    for ext in EXTENSIONS {
        let with_ext = path.with_extension(ext);
        if with_ext.is_file() {
            return with_ext.canonicalize().ok().or(Some(with_ext));
        }
        let index = path.join(format!("index.{ext}"));
        if index.is_file() {
            return Some(index);
        }
    }
    None
}

fn is_source(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| EXTENSIONS.contains(&ext))
}
