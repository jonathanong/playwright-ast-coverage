use super::RuleConfig;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct ProjectConfig {
    pub root: Option<String>,
    pub rules: Vec<String>,
}

pub(super) fn roots_for_rule(
    projects: &HashMap<String, ProjectConfig>,
    rules: &HashMap<String, RuleConfig>,
    root: &Path,
    rule_id: &str,
) -> Vec<PathBuf> {
    if rules.get(rule_id).is_some_and(|rule| !rule.enabled) {
        return Vec::new();
    }

    let project_roots = projects
        .values()
        .filter(|project| project.rules.iter().any(|rule| rule == rule_id))
        .map(|project| {
            project
                .root
                .as_deref()
                .map(|project_root| root.join(project_root))
                .unwrap_or_else(|| root.to_path_buf())
        })
        .collect::<Vec<_>>();
    if !project_roots.is_empty() {
        return project_roots;
    }

    if projects.is_empty() || rules.contains_key(rule_id) {
        vec![root.to_path_buf()]
    } else {
        Vec::new()
    }
}
