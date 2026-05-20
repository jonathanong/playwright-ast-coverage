use super::RuleConfig;
use crate::config::v2::schema::ProjectType;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
#[serde(default)]
pub struct ProjectConfig {
    #[serde(rename = "type")]
    pub type_: Option<ProjectType>,
    pub root: Option<String>,
    pub include: Vec<String>,
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
        if let Some(project_root) = project.effective_root(root) {
            project_roots.push(project_root);
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

impl ProjectConfig {
    pub fn effective_root(&self, workspace_root: &Path) -> Option<PathBuf> {
        match self.root.as_deref() {
            Some(root) => Some(workspace_root.join(root)),
            None if self.type_ == Some(ProjectType::Nextjs) => inferred_nextjs_root(workspace_root),
            None => None,
        }
    }
}

fn inferred_nextjs_root(root: &Path) -> Option<PathBuf> {
    let mut roots = crate::codebase::ts_source::discover_with_basenames(
        root,
        &[],
        &[
            "next.config.js",
            "next.config.mjs",
            "next.config.ts",
            "next.config.mts",
        ],
    )
    .into_iter()
    .filter_map(|path| path.parent().map(Path::to_path_buf))
    .collect::<Vec<_>>();
    roots.sort();
    roots.dedup();
    match roots.as_slice() {
        [root] => Some(root.clone()),
        _ => None,
    }
}
