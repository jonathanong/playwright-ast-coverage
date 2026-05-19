use super::RuleFinding;
use crate::codebase::ts_source::{
    discover_with_extensions, has_disable_file_comment, relative_slash_path,
};
use crate::config::v2::NoMistakesConfig;
use anyhow::Result;
use rayon::prelude::*;
use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const RULE_ID: &str = "rust-max-lines-per-file";

const DEFAULT_SRC_MAX: usize = 200;
const DEFAULT_TEST_MAX: usize = 500;

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct Options {
    pub(crate) src_max: Option<usize>,
    pub(crate) test_max: Option<usize>,
    pub(crate) excludes: Vec<String>,
    pub(crate) roots: Option<Vec<PathBuf>>,
}

pub fn check(root: &Path, config: &NoMistakesConfig) -> Result<Vec<RuleFinding>> {
    let opts = parse_opts(config);
    let skip = &config.filesystem.skip_directories;
    let roots = normalize_roots(&opts, root);
    let files: Vec<PathBuf> = roots
        .iter()
        .flat_map(|r| discover_with_extensions(r, skip, &["rs"]))
        .filter(|p| !is_excluded(root, p, &opts.excludes))
        .collect();
    scan(root, &opts, &files)
}

/// Check using a pre-discovered file list to avoid a second filesystem walk.
pub(crate) fn check_with_files(
    root: &Path,
    config: &NoMistakesConfig,
    all_files: &[PathBuf],
) -> Result<Vec<RuleFinding>> {
    let opts = parse_opts(config);
    let roots = normalize_roots(&opts, root);
    let files: Vec<PathBuf> = all_files
        .iter()
        .filter(|p| {
            roots.iter().any(|r| p.starts_with(r))
                && p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e == "rs")
                && !is_excluded(root, p, &opts.excludes)
        })
        .cloned()
        .collect();
    scan(root, &opts, &files)
}

fn parse_opts(config: &NoMistakesConfig) -> Options {
    config
        .rules
        .get(RULE_ID)
        .map_or_else(Default::default, |r| r.rule_options())
}

fn normalize_roots(opts: &Options, root: &Path) -> Vec<PathBuf> {
    opts.roots
        .as_deref()
        .map(|rs| {
            rs.iter()
                .map(|r| {
                    if r.is_absolute() {
                        r.clone()
                    } else {
                        root.join(r)
                    }
                })
                .collect()
        })
        .unwrap_or_else(|| vec![root.to_path_buf()])
}

fn is_excluded(root: &Path, path: &Path, excludes: &[String]) -> bool {
    let rel = path.strip_prefix(root).unwrap_or(path).to_string_lossy();
    excludes.iter().any(|e| rel.contains(e.as_str()))
}

fn scan(root: &Path, opts: &Options, files: &[PathBuf]) -> Result<Vec<RuleFinding>> {
    let src_max = opts.src_max.unwrap_or(DEFAULT_SRC_MAX);
    let test_max = opts.test_max.unwrap_or(DEFAULT_TEST_MAX);
    let mut findings: Vec<RuleFinding> = files
        .par_iter()
        .flat_map(|path| {
            let limit = if is_test_file(root, path) {
                test_max
            } else {
                src_max
            };
            check_file(path, root, limit)
        })
        .collect();
    findings.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(findings)
}

fn check_file(path: &Path, root: &Path, limit: usize) -> Option<RuleFinding> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return None;
    };
    if has_disable_file_comment(&content, RULE_ID) {
        return None;
    }
    let code_lines = count_code_lines(&content);
    if code_lines <= limit {
        return None;
    }
    Some(RuleFinding {
        rule: RULE_ID.to_string(),
        file: relative_slash_path(root, path),
        line: 1,
        message: format!("{code_lines} code lines (max {limit}) - split into smaller modules"),
        import: None,
        target: None,
    })
}

pub(crate) fn is_test_file(root: &Path, path: &Path) -> bool {
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    rel.contains("/tests/")
        || rel.starts_with("tests/")
        || path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n == "tests.rs")
}

pub(crate) use line_counter::count_code_lines;

mod line_counter;

#[cfg(test)]
mod tests;
