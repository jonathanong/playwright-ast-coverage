use no_mistakes_core::config::v2::NoMistakesConfig;
use std::path::{Path, PathBuf};

pub(crate) fn discover_check_files(
    root: &Path,
    config: &NoMistakesConfig,
    skip_directories: &[String],
    unique_exports_enabled: bool,
) -> Vec<PathBuf> {
    let mut files = no_mistakes_core::codebase::ts_source::discover_files(root, skip_directories);
    if unique_exports_enabled {
        for project_root in unique_exports_project_roots(root, config) {
            files.extend(no_mistakes_core::codebase::ts_source::discover_files(
                &project_root,
                skip_directories,
            ));
        }
    }
    files.sort();
    files.dedup();
    files
}

fn unique_exports_project_roots(root: &Path, config: &NoMistakesConfig) -> Vec<PathBuf> {
    let rule_id = no_mistakes_core::codebase::unique_exports::RULE_ID;
    let mut roots = Vec::new();
    for rule in config.rule_applications(rule_id) {
        if rule.applies_to_repository() {
            roots.push(root.to_path_buf());
        }
        for project_name in &rule.projects {
            let Some(project) = config.projects.get(project_name) else {
                continue;
            };
            if let Some(project_root) = project_root(root, project) {
                roots.push(project_root);
            }
        }
    }
    roots.sort();
    roots.dedup();
    roots
}

fn project_root(
    root: &Path,
    project: &no_mistakes_core::config::v2::schema::Project,
) -> Option<PathBuf> {
    if let Some(project_root) = project.root.as_deref() {
        return Some(root.join(project_root));
    }
    if project.type_ == Some(no_mistakes_core::config::v2::schema::ProjectType::Nextjs) {
        return inferred_nextjs_root(root);
    }
    Some(root.to_path_buf())
}

fn inferred_nextjs_root(root: &Path) -> Option<PathBuf> {
    let mut roots = no_mistakes_core::codebase::ts_source::discover_with_basenames(
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

#[cfg(test)]
mod tests;
