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
    for project in config.projects.values() {
        if project.rules.iter().any(|rule| rule == rule_id) {
            roots.push(root.join(project.root.as_deref().unwrap_or("")));
        }
    }
    if !roots.is_empty() {
        return roots;
    }
    vec![root.to_path_buf()]
}
