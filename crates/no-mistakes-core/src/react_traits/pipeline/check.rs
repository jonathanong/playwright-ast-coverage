use crate::react_traits::report::types::{FileConfig, RootConfig, Violation};
use anyhow::Result;
use std::path::Path;

pub fn run_check(
    root: &Path,
    config_path: Option<&Path>,
    targets: &[String],
    assert_no_fetch: bool,
) -> Result<Vec<Violation>> {
    let stems = [".no-mistakes", ".react-traits"];
    let root_config: RootConfig = crate::config::load_config(root, config_path, &stems)?;
    let file_config = build_file_config(root_config);
    let effective_no_fetch = assert_no_fetch || file_config.assert_no_fetch.unwrap_or(false);
    let facts_list =
        crate::react_traits::pipeline::run::run_analyze_inner(root, &file_config, targets, None)?;
    let mut violations = Vec::new();
    for facts in &facts_list {
        if effective_no_fetch {
            let has_fetch = !facts.fetches.is_empty()
                || facts
                    .inherited_from_children
                    .as_ref()
                    .is_some_and(|agg| agg.has_fetch);
            if has_fetch {
                violations.push(Violation {
                    component: facts.name.clone(),
                    file: facts.file.clone(),
                    rule: "assert-no-fetch".to_string(),
                    detail: facts.fetches.first().and_then(|f| f.shape.clone()),
                });
            }
        }
    }
    Ok(violations)
}

fn build_file_config(root_config: RootConfig) -> FileConfig {
    let mut file_config = root_config.legacy;
    if let Some(overrides) = root_config.react_traits {
        if overrides.frontend_root.is_some() {
            file_config.frontend_root = overrides.frontend_root;
        }
        if overrides.assert_no_fetch.is_some() {
            file_config.assert_no_fetch = overrides.assert_no_fetch;
        }
    }
    file_config
}
