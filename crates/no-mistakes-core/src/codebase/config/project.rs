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

    let mut project_roots = Vec::new();
    for project in projects.values() {
        if !project.rules.iter().any(|rule| rule == rule_id) {
            continue;
        }
        if let Some(project_root) = project.root.as_deref() {
            project_roots.push(root.join(project_root));
        } else {
            project_roots.push(root.to_path_buf());
        }
    }
    if !project_roots.is_empty() {
        return project_roots;
    }

    if projects.is_empty() || rules.contains_key(rule_id) {
        vec![root.to_path_buf()]
    } else {
        Vec::new()
    }
}
