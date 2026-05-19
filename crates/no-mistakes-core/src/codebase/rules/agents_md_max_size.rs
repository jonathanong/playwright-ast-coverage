use super::RuleFinding;
use crate::codebase::ts_source::{
    discover_with_basenames, has_disable_file_comment, relative_slash_path,
};
use crate::config::v2::NoMistakesConfig;
use anyhow::Result;
use rayon::prelude::*;
use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const RULE_ID: &str = "agents-md-max-size";

const DEFAULT_MAX_LINES: usize = 200;
const DEFAULT_MAX_CHARS: usize = 12_000;
const DEFAULT_FILENAMES: &[&str] = &["AGENTS.md", "CLAUDE.md"];

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct Options {
    pub(crate) max_lines: Option<usize>,
    pub(crate) max_chars: Option<usize>,
    pub(crate) filenames: Option<Vec<String>>,
    pub(crate) roots: Option<Vec<PathBuf>>,
}

pub fn check(root: &Path, config: &NoMistakesConfig) -> Result<Vec<RuleFinding>> {
    let opts = parse_opts(config);
    let filenames = filenames_from_opts(&opts);
    let skip = &config.filesystem.skip_directories;
    let roots = roots_from_opts(&opts, root);
    let files: Vec<PathBuf> = roots
        .iter()
        .flat_map(|r| discover_with_basenames(r, skip, &filenames))
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
    let filenames = filenames_from_opts(&opts);
    let files: Vec<&PathBuf> = all_files
        .iter()
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| filenames.contains(&n))
        })
        .collect();
    scan(root, &opts, &files.into_iter().cloned().collect::<Vec<_>>())
}

fn parse_opts(config: &NoMistakesConfig) -> Options {
    config
        .rules
        .get(RULE_ID)
        .map_or_else(Default::default, |r| r.rule_options())
}

fn filenames_from_opts<'a>(opts: &'a Options) -> Vec<&'a str> {
    opts.filenames
        .as_deref()
        .map(|v| v.iter().map(String::as_str).collect())
        .unwrap_or_else(|| DEFAULT_FILENAMES.to_vec())
}

fn roots_from_opts(opts: &Options, root: &Path) -> Vec<PathBuf> {
    opts.roots
        .clone()
        .unwrap_or_else(|| vec![root.to_path_buf()])
}

fn scan(root: &Path, opts: &Options, files: &[PathBuf]) -> Result<Vec<RuleFinding>> {
    let max_lines = opts.max_lines.unwrap_or(DEFAULT_MAX_LINES);
    let max_chars = opts.max_chars.unwrap_or(DEFAULT_MAX_CHARS);
    let mut findings: Vec<RuleFinding> = files
        .par_iter()
        .flat_map(|path| check_file(path, root, max_lines, max_chars))
        .collect();
    findings.sort_by(|a, b| a.file.cmp(&b.file).then(a.message.cmp(&b.message)));
    Ok(findings)
}

fn check_file(path: &Path, root: &Path, max_lines: usize, max_chars: usize) -> Vec<RuleFinding> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    if has_disable_file_comment(&content, RULE_ID) {
        return Vec::new();
    }
    let file = relative_slash_path(root, path);
    let mut findings = Vec::new();
    let line_count = count_lines(&content);
    if line_count > max_lines {
        findings.push(RuleFinding {
            rule: RULE_ID.to_string(),
            file: file.clone(),
            line: 1,
            message: format!(
                "{line_count} lines (max {max_lines}) - trim to keep agent context lean"
            ),
            import: None,
            target: None,
        });
    }
    let char_count = content.chars().count();
    if char_count > max_chars {
        findings.push(RuleFinding {
            rule: RULE_ID.to_string(),
            file,
            line: 1,
            message: format!(
                "{char_count} characters (max {max_chars}) - trim to keep agent context lean"
            ),
            import: None,
            target: None,
        });
    }
    findings
}

pub(crate) fn count_lines(content: &str) -> usize {
    if content.is_empty() {
        return 0;
    }
    let newlines = content.bytes().filter(|&b| b == b'\n').count();
    if content.ends_with('\n') {
        newlines
    } else {
        newlines + 1
    }
}

#[cfg(test)]
mod tests;
