use ignore::WalkBuilder;
use oxc::ast::ast::{Expression, PropertyKey};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

pub mod facts;
pub mod jsx;

pub const TS_JS_EXTENSIONS: &[&str] = &["js", "jsx", "mjs", "mts", "cjs", "cts", "ts", "tsx"];

pub const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "dist",
    ".git",
    ".next",
    "coverage",
    "fixtures",
    "target",
    "build",
];

pub fn is_skipped_dir(name: &str) -> bool {
    SKIP_DIRS.contains(&name)
}

/// Walk all non-ignored files under `root`.
///
/// Uses the `ignore` crate so `.gitignore` rules and hidden directories are
/// excluded, except `.github` because CI workflow analysis needs those files
/// when no `.git` metadata is available. `node_modules` is also always excluded
/// as a safety net for repos where it is not gitignored.
///
/// `extra_skip` is an optional list of additional directory names to prune
/// (e.g. `config.filesystem.skip_directories`).
pub fn walk_files(root: &Path, extra_skip: &[String]) -> Vec<PathBuf> {
    let extra_skip: HashSet<String> = extra_skip.iter().cloned().collect();
    WalkBuilder::new(root)
        .hidden(false)
        .filter_entry(move |e| {
            // depth==0 is the walk root itself; never prune it by name.
            // Only prune directories — never exclude individual files by name.
            if e.depth() == 0 {
                return true;
            }
            let name = e.file_name().to_str().unwrap_or("");
            if e.file_type().is_some_and(|ft| ft.is_dir()) {
                !SKIP_DIRS.contains(&name)
                    && !extra_skip.contains(name)
                    && !is_hidden_non_github(name)
            } else {
                !is_hidden_non_github(name)
            }
        })
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
        .map(|e| normalize_discovery_path(e.path()))
        .collect()
}

fn is_hidden_non_github(name: &str) -> bool {
    name.starts_with('.') && name != ".github"
}

/// Return all tracked and untracked non-ignored files under `root`.
///
/// This follows the repo-wide convention that git is the source of truth for
/// file discovery: tracked files plus untracked files that are not hidden by
/// `.gitignore`. The result is repo-relative, sorted, and deduplicated.
pub fn git_visible_files(root: &Path) -> Option<Vec<String>> {
    let tracked = git_ls_files(root, false)?;
    let untracked = git_ls_files(root, true)?;
    let mut combined: Vec<String> = tracked.into_iter().chain(untracked).collect();
    combined.sort();
    combined.dedup();
    Some(combined)
}

/// Return git-visible files as absolute paths. Falls back to the ignore-based
/// walker outside git repositories so unit tests and ad-hoc directories still
/// behave sensibly.
pub fn discover_files(root: &Path, extra_skip: &[String]) -> Vec<PathBuf> {
    let root = normalize_discovery_path(root);
    match git_visible_files(&root) {
        Some(files) => files
            .into_iter()
            .map(|rel| normalize_discovery_path(&root.join(rel)))
            .filter(|p| p.exists())
            .filter(|p| !is_under_skipped_dir(&root, p, extra_skip))
            .collect(),
        None => walk_files(&root, extra_skip),
    }
}

pub fn discover_source_files(root: &Path, extra_skip: &[String]) -> Vec<PathBuf> {
    discover_files(root, extra_skip)
        .into_iter()
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| TS_JS_EXTENSIONS.contains(&ext))
        })
        .collect()
}

pub fn relative_slash_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub fn line_number(source: &str, start: u32) -> usize {
    byte_offset_to_line(source, start as usize) as usize
}

fn normalize_discovery_path(path: &Path) -> PathBuf {
    let normalized = crate::codebase::ts_resolver::normalize_path(path);
    if normalized.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        normalized
    }
}

fn is_under_skipped_dir(root: &Path, path: &Path, extra_skip: &[String]) -> bool {
    let extra_skip: HashSet<&str> = extra_skip.iter().map(String::as_str).collect();
    path.strip_prefix(root).ok().is_some_and(|rel| {
        rel.components().any(|component| {
            component
                .as_os_str()
                .to_str()
                .is_some_and(|name| SKIP_DIRS.contains(&name) || extra_skip.contains(name))
        })
    })
}

fn git_ls_files(root: &Path, others: bool) -> Option<Vec<String>> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(root).arg("ls-files");
    cmd.env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    if others {
        cmd.arg("--others").arg("--exclude-standard");
    }
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8(out.stdout).ok()?;
    Some(
        stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect(),
    )
}

pub fn byte_offset_to_line(source: &str, byte_offset: usize) -> u32 {
    let end = byte_offset.min(source.len());
    let line = source[..end].bytes().filter(|&b| b == b'\n').count();
    (line + 1) as u32
}

/// Returns `true` if the line immediately before `stmt_line` (1-based) contains
/// a `guardrails-disable-next-line <rule_id>` directive comment.
///
/// Matches:
/// - `// guardrails-disable-next-line <rule_id>`
/// - `// guardrails-disable-next-line <rule_id>: <reason>`
/// - `// guardrails-disable-next-line <rule_id> <reason>`
pub fn has_disable_comment(source: &str, stmt_line: u32, rule_id: &str) -> bool {
    if stmt_line < 2 {
        return false;
    }
    source
        .lines()
        .nth((stmt_line - 2) as usize)
        .map(|line| {
            let trimmed = line.trim();
            if !trimmed.starts_with("//") {
                return false;
            }
            let rest = trimmed
                .strip_prefix("//")
                .expect("line starts with //")
                .trim();
            let Some(after_directive) = rest.strip_prefix("guardrails-disable-next-line ") else {
                return false;
            };
            let rule_part = after_directive.trim();
            rule_part.strip_prefix(rule_id).is_some_and(|suffix| {
                suffix.is_empty()
                    || suffix.starts_with(':')
                    || suffix.starts_with(char::is_whitespace)
            })
        })
        .unwrap_or(false)
}

/// Returns `true` if a leading comment disables `rule_id` for the whole file.
///
/// Matches:
/// - `// guardrails-disable-file <rule_id>`
/// - `// guardrails-disable-file <rule_id>: <reason>`
/// - `// guardrails-disable-file <rule_id> <reason>`
pub fn has_disable_file_comment(source: &str, rule_id: &str) -> bool {
    let mut in_block_comment = false;

    for line in source.trim_start_matches('\u{FEFF}').lines() {
        let mut rest = line.trim();

        loop {
            if rest.is_empty() {
                break;
            }

            if in_block_comment {
                let Some(end) = rest.find("*/") else {
                    break;
                };
                in_block_comment = false;
                rest = rest[end + 2..].trim();
                continue;
            }

            if rest.starts_with("/*") {
                let Some(end) = rest.find("*/") else {
                    in_block_comment = true;
                    break;
                };
                rest = rest[end + 2..].trim();
                continue;
            }

            let Some(rest) = rest.strip_prefix("//").map(|s| s.trim()) else {
                return false;
            };
            let Some(after_directive) = rest.strip_prefix("guardrails-disable-file ") else {
                break;
            };
            let rule_part = after_directive.trim();
            if rule_part.strip_prefix(rule_id).is_some_and(|suffix| {
                suffix.is_empty()
                    || suffix.starts_with(':')
                    || suffix.starts_with(char::is_whitespace)
            }) {
                return true;
            }
            break;
        }
    }

    false
}

pub fn unwrap_ts_wrappers<'a>(expr: &'a Expression<'a>) -> &'a Expression<'a> {
    match expr {
        Expression::TSAsExpression(e) => unwrap_ts_wrappers(&e.expression),
        Expression::TSNonNullExpression(e) => unwrap_ts_wrappers(&e.expression),
        Expression::TSTypeAssertion(e) => unwrap_ts_wrappers(&e.expression),
        Expression::TSSatisfiesExpression(e) => unwrap_ts_wrappers(&e.expression),
        Expression::ParenthesizedExpression(e) => unwrap_ts_wrappers(&e.expression),
        other => other,
    }
}

pub fn static_property_key_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
        PropertyKey::StringLiteral(s) => Some(s.value.as_str()),
        _ => None,
    }
}

/// Returns `true` if the first non-empty line of `source` is a `'use client'` or
/// `"use client"` directive prologue. Checks only the first line to avoid false positives
/// from occurrences inside comments or string literals later in the file.
pub fn starts_with_use_client(source: &str) -> bool {
    let first_line = source
        .trim_start_matches('\u{FEFF}') // strip optional BOM
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("");
    matches!(
        first_line.trim(),
        "'use client'" | "'use client';" | "\"use client\"" | "\"use client\";"
    )
}

/// Returns `true` if `relative` is a test file — either living under an
/// `/__tests__/` directory or having a `.test.*` / `.spec.*` suffix that matches
/// the pattern `\.(test|spec)\.[cm]?[jt]sx?`.
pub fn is_test_file(relative: &str) -> bool {
    if relative.contains("/__tests__/") {
        return true;
    }
    const SUFFIXES: &[&str] = &[
        ".test.ts",
        ".test.tsx",
        ".test.js",
        ".test.jsx",
        ".test.mts",
        ".test.cts",
        ".test.mjs",
        ".test.cjs",
        ".spec.ts",
        ".spec.tsx",
        ".spec.js",
        ".spec.jsx",
        ".spec.mts",
        ".spec.cts",
        ".spec.mjs",
        ".spec.cjs",
    ];
    SUFFIXES.iter().any(|s| relative.ends_with(s))
}

#[cfg(test)]
mod tests;
