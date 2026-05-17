pub mod test_no_unmocked_dynamic_imports;

use anyhow::Result;
use serde::Serialize;
use std::path::Path;

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

fn rule_enabled(config: &crate::config::v2::NoMistakesConfig, rule_id: &str) -> bool {
    let top_level = config
        .rules
        .get(rule_id)
        .map(|rule| rule.enabled)
        .unwrap_or(false);
    top_level
        || config.projects.values().any(|project| {
            project.rules.iter().any(|rule| rule == rule_id)
                && config
                    .rules
                    .get(rule_id)
                    .map(|rule| rule.enabled)
                    .unwrap_or(true)
        })
}

#[cfg(test)]
mod tests;
