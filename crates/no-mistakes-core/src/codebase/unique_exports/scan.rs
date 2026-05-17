use super::{SourceFile, RULE_ID};
use crate::codebase::ts_resolver::normalize_path;
use crate::codebase::ts_source::{has_disable_file_comment, relative_slash_path, TS_JS_EXTENSIONS};
use crate::codebase::ts_symbols::extract_symbols;
use anyhow::Context;
use rayon::prelude::*;
use regex::Regex;
use std::path::{Path, PathBuf};

pub(super) fn filter_source_files(
    root: &Path,
    files: Vec<PathBuf>,
    skip_file_patterns: &[String],
) -> Vec<PathBuf> {
    let patterns: Vec<Regex> = skip_file_patterns
        .iter()
        .filter_map(|pattern| Regex::new(pattern).ok())
        .collect();
    files
        .into_iter()
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| TS_JS_EXTENSIONS.contains(&ext))
        })
        .filter(|path| {
            let rel = relative_slash_path(root, path);
            !patterns.iter().any(|pattern| pattern.is_match(&rel))
        })
        .collect()
}

pub(super) fn collect_source_files(root: &Path, files: &[PathBuf]) -> Vec<SourceFile> {
    files
        .par_iter()
        .filter_map(|path| {
            let source = std::fs::read_to_string(path).ok()?;
            let is_tsx = matches!(
                path.extension().and_then(|ext| ext.to_str()),
                Some("tsx" | "jsx")
            );
            let symbols = extract_symbols(&source, is_tsx)
                .with_context(|| format!("extracting symbols from {}", path.display()))
                .ok()?;
            Some(SourceFile {
                path: normalize_path(path),
                rel: relative_slash_path(root, path),
                disabled: has_disable_file_comment(&source, RULE_ID),
                is_nextjs_project: file_is_in_nextjs_project(root, path),
                source,
                symbols,
            })
        })
        .collect()
}

pub(super) fn file_is_in_nextjs_project(root: &Path, path: &Path) -> bool {
    let root = normalize_path(root);
    let mut current = match path.parent() {
        Some(parent) => normalize_path(parent),
        None => root.clone(),
    };
    loop {
        if package_json_has_next_dependency(&current.join("package.json")) {
            return true;
        }
        if current == root || !current.pop() {
            return false;
        }
    }
}

pub(super) fn package_json_has_next_dependency(path: &Path) -> bool {
    let Ok(source) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(package_json) = serde_json::from_str::<serde_json::Value>(&source) else {
        return false;
    };
    for field in ["dependencies", "devDependencies", "peerDependencies"] {
        let Some(dependencies) = package_json.get(field).and_then(|value| value.as_object()) else {
            continue;
        };
        if !dependencies.contains_key("next") {
            continue;
        }
        return true;
    }
    false
}

pub(super) fn sorted_paths<'a>(paths: impl Iterator<Item = &'a PathBuf>) -> Vec<&'a PathBuf> {
    let mut paths: Vec<_> = paths.collect();
    paths.sort();
    paths
}
