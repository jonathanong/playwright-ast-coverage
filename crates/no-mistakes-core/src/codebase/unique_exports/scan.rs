use super::{SourceFile, RULE_ID};
use crate::codebase::ts_resolver::normalize_path;
use crate::codebase::ts_source::{has_disable_file_comment, relative_slash_path, TS_JS_EXTENSIONS};
use anyhow::{Context, Result};
#[cfg(test)]
use rayon::prelude::*;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub(super) fn filter_source_files(
    root: &Path,
    files: &[PathBuf],
    skip_file_patterns: &[String],
) -> Result<Vec<PathBuf>> {
    let patterns: Vec<Regex> = skip_file_patterns
        .iter()
        .map(|pattern| {
            Regex::new(pattern).with_context(|| format!("invalid skip file pattern: {pattern}"))
        })
        .collect::<Result<_>>()?;
    Ok(files
        .iter()
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| TS_JS_EXTENSIONS.contains(&ext))
        })
        .filter(|path| {
            let rel = relative_slash_path(root, path);
            !patterns.iter().any(|pattern| pattern.is_match(&rel))
        })
        .cloned()
        .collect())
}

#[cfg(test)]
pub(super) fn collect_source_files(root: &Path, files: &[PathBuf]) -> Result<Vec<SourceFile>> {
    let nextjs_projects = NextJsProjectLookup::new(root, files);
    files
        .par_iter()
        .map(|path| {
            let source = std::fs::read_to_string(path)
                .with_context(|| format!("reading source file {}", path.display()))?;
            let is_tsx = matches!(
                path.extension().and_then(|ext| ext.to_str()),
                Some("tsx" | "jsx")
            );
            let disabled = has_disable_file_comment(&source, RULE_ID);
            let symbols = if disabled {
                Default::default()
            } else {
                crate::codebase::ts_symbols::extract_symbols(&source, is_tsx)
                    .with_context(|| format!("extracting symbols from {}", path.display()))?
            };
            Ok(SourceFile {
                path: normalize_path(path),
                rel: relative_slash_path(root, path),
                disabled,
                is_nextjs_project: nextjs_projects.contains_file(path),
                source,
                symbols,
            })
        })
        .collect()
}

pub(super) fn collect_source_files_from_facts(
    root: &Path,
    files: &[PathBuf],
    shared: &crate::codebase::check_facts::CheckFactMap,
) -> Result<Vec<SourceFile>> {
    let nextjs_projects = NextJsProjectLookup::new(root, files);
    let mut source_files = Vec::new();
    for path in files {
        let Some(facts) = shared.ts.get(path) else {
            anyhow::bail!("missing shared facts for {}", path.display());
        };
        let Some(source) = facts.source.clone() else {
            anyhow::bail!("missing source facts for {}", path.display());
        };
        let disabled = has_disable_file_comment(&source, RULE_ID);
        if !disabled {
            if let Some(error) = &facts.parse_error {
                anyhow::bail!("failed to parse {}: {error}", path.display());
            }
        }
        let symbols = if disabled {
            Default::default()
        } else {
            let Some(symbols) = facts.symbols.clone() else {
                anyhow::bail!("missing symbol facts for {}", path.display());
            };
            symbols
        };
        source_files.push(SourceFile {
            path: normalize_path(path),
            rel: relative_slash_path(root, path),
            disabled,
            is_nextjs_project: nextjs_projects.contains_file(path),
            source,
            symbols,
        });
    }
    Ok(source_files)
}

pub(super) struct NextJsProjectLookup {
    directories: HashMap<PathBuf, bool>,
}

impl NextJsProjectLookup {
    pub(super) fn new(root: &Path, files: &[PathBuf]) -> Self {
        let root = normalize_path(root);
        let mut directories = HashSet::from([root.clone()]);
        for path in files {
            let mut current = path
                .parent()
                .map(normalize_path)
                .unwrap_or_else(|| root.clone());
            loop {
                directories.insert(current.clone());
                if current == root || !current.pop() {
                    break;
                }
            }
        }

        let mut sorted: Vec<_> = directories.into_iter().collect();
        sorted.sort_by_key(|path| path.components().count());
        let mut directories = HashMap::new();
        for directory in sorted {
            let parent_is_nextjs = directory
                .parent()
                .and_then(|parent| directories.get(&normalize_path(parent)))
                .copied()
                .unwrap_or(false);
            directories.insert(
                directory.clone(),
                parent_is_nextjs
                    || package_json_has_next_dependency(&directory.join("package.json")),
            );
        }
        Self { directories }
    }

    pub(super) fn contains_file(&self, path: &Path) -> bool {
        path.parent()
            .map(normalize_path)
            .and_then(|directory| self.directories.get(&directory).copied())
            .unwrap_or(false)
    }
}

#[cfg(test)]
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
