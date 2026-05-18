use crate::react_traits::report::types::{RootConfig, Violation};
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
    let file_config = root_config.into_file_config();
    let effective_no_fetch = assert_no_fetch || file_config.assert_no_fetch.unwrap_or(false);
    if !effective_no_fetch {
        return Ok(Vec::new());
    }
    let facts_list =
        crate::react_traits::pipeline::run::run_analyze_inner(root, &file_config, targets, None)?;
    Ok(assert_no_fetch_violations(&facts_list))
}

pub fn run_check_with_facts(
    root: &Path,
    config_path: Option<&Path>,
    targets: &[String],
    assert_no_fetch: bool,
    shared: &crate::codebase::check_facts::CheckFactMap,
) -> Result<Vec<Violation>> {
    let stems = [".no-mistakes", ".react-traits"];
    let root_config: RootConfig = crate::config::load_config(root, config_path, &stems)?;
    let file_config = root_config.into_file_config();
    let effective_no_fetch = assert_no_fetch || file_config.assert_no_fetch.unwrap_or(false);
    if !effective_no_fetch {
        return Ok(Vec::new());
    }
    let facts_list = crate::react_traits::pipeline::run_with_facts::run_analyze_inner_with_facts(
        root,
        &file_config,
        targets,
        shared,
    )?;
    Ok(assert_no_fetch_violations(&facts_list))
}

fn assert_no_fetch_violations(
    facts_list: &[crate::react_traits::ComponentFacts],
) -> Vec<Violation> {
    let mut violations = Vec::new();
    for facts in facts_list {
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
    violations
}

pub fn check_enabled(
    root: &Path,
    config_path: Option<&Path>,
    assert_no_fetch: bool,
) -> Result<bool> {
    let stems = [".no-mistakes", ".react-traits"];
    let root_config: RootConfig = crate::config::load_config(root, config_path, &stems)?;
    let file_config = root_config.into_file_config();
    Ok(assert_no_fetch || file_config.assert_no_fetch.unwrap_or(false))
}

#[cfg(test)]
mod tests;
