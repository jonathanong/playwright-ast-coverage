pub mod agents_md_max_size;
pub mod rust_max_lines_per_file;
pub mod rust_no_inline_tests;
pub mod test_no_unmocked_dynamic_imports;

use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub use agents_md_max_size::RULE_ID as AGENTS_MD_MAX_SIZE;
pub use rust_max_lines_per_file::RULE_ID as RUST_MAX_LINES_PER_FILE;
pub use rust_no_inline_tests::RULE_ID as RUST_NO_INLINE_TESTS;
pub use test_no_unmocked_dynamic_imports::RULE_ID as TEST_NO_UNMOCKED_DYNAMIC_IMPORTS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleFinding {
    pub rule: String,
    pub file: String,
    pub line: usize,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

pub fn run_check(
    root: &Path,
    config_path: Option<&Path>,
    tsconfig_path: Option<&Path>,
) -> Result<Vec<RuleFinding>> {
    let config = crate::config::v2::load_v2_config(root, config_path)?;
    if !rule_enabled(&config, TEST_NO_UNMOCKED_DYNAMIC_IMPORTS) {
        return Ok(Vec::new());
    }
    test_no_unmocked_dynamic_imports::check(root, &config, tsconfig_path)
}

pub fn run_check_with_facts(
    root: &Path,
    config_path: Option<&Path>,
    tsconfig_path: Option<&Path>,
    shared: &crate::codebase::check_facts::CheckFactMap,
) -> Result<Vec<RuleFinding>> {
    let config = crate::config::v2::load_v2_config(root, config_path)?;
    if !rule_enabled(&config, TEST_NO_UNMOCKED_DYNAMIC_IMPORTS) {
        return Ok(Vec::new());
    }
    test_no_unmocked_dynamic_imports::check_with_facts(root, &config, tsconfig_path, shared)
}

/// Run the three filesystem rules using a pre-discovered file list so the
/// caller's single `git ls-files` / walker result is reused — no second walk.
pub fn run_filesystem_rules_with_files(
    root: &Path,
    config_path: Option<&Path>,
    files: &[PathBuf],
) -> Result<Vec<RuleFinding>> {
    let config = crate::config::v2::load_v2_config(root, config_path)?;
    let mut findings = Vec::new();
    if rule_enabled(&config, AGENTS_MD_MAX_SIZE) {
        findings.extend(agents_md_max_size::check_with_files(root, &config, files)?);
    }
    if rule_enabled(&config, RUST_MAX_LINES_PER_FILE) {
        let rule_findings = rust_max_lines_per_file::check_with_files(root, &config, files)?;
        findings.extend(rule_findings);
    }
    if rule_enabled(&config, RUST_NO_INLINE_TESTS) {
        let rule_findings = rust_no_inline_tests::check_with_files(root, &config, files)?;
        findings.extend(rule_findings);
    }
    Ok(findings)
}

/// Standalone entry point (used by tests / direct invocations without a
/// pre-discovered file list). Each rule does its own discovery.
pub fn run_filesystem_rules(root: &Path, config_path: Option<&Path>) -> Result<Vec<RuleFinding>> {
    let config = crate::config::v2::load_v2_config(root, config_path)?;
    let mut findings = Vec::new();
    if rule_enabled(&config, AGENTS_MD_MAX_SIZE) {
        findings.extend(agents_md_max_size::check(root, &config)?);
    }
    if rule_enabled(&config, RUST_MAX_LINES_PER_FILE) {
        findings.extend(rust_max_lines_per_file::check(root, &config)?);
    }
    if rule_enabled(&config, RUST_NO_INLINE_TESTS) {
        findings.extend(rust_no_inline_tests::check(root, &config)?);
    }
    Ok(findings)
}

pub(crate) fn rule_enabled(config: &crate::config::v2::NoMistakesConfig, rule_id: &str) -> bool {
    config.rule_configured(rule_id)
}

#[cfg(test)]
mod tests;
