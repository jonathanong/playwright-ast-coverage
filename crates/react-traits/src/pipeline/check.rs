use crate::cli::Cli;
use crate::report::types::Violation;
use anyhow::Result;
use std::path::Path;

#[cfg(test)]
mod tests;

pub(crate) fn run_check(
    base_root: &Path,
    cli: &Cli,
    targets: &[String],
    assert_no_fetch: bool,
) -> Result<Vec<Violation>> {
    let (_root, file_config) = crate::pipeline::run::load_root_and_config(base_root, cli)?;
    let effective_no_fetch = assert_no_fetch || file_config.assert_no_fetch.unwrap_or(false);
    let facts_list = crate::pipeline::run::run_analyze(base_root, cli, targets, None)?;
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
