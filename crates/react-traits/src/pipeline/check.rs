use crate::analyze::file::analyze_file;
use crate::cli::Cli;
use crate::pipeline::glob::expand_globs;
use crate::report::types::{RootConfig, Violation};
use anyhow::Result;
use no_mistakes_core::config;
use std::path::Path;

pub(crate) fn run_check(
    base_root: &Path,
    cli: &Cli,
    targets: &[String],
    assert_no_fetch: bool,
) -> Result<Vec<Violation>> {
    let root = base_root.join(&cli.root);
    let stems = [".no-mistakes", ".react-traits"];
    let root_config: RootConfig = config::load_config(&root, cli.config.as_deref(), &stems)?;
    let file_config = root_config.react_traits.unwrap_or(root_config.legacy);
    let effective_no_fetch = assert_no_fetch || file_config.assert_no_fetch.unwrap_or(false);

    let frontend_root = root.join(file_config.frontend_root.as_deref().unwrap_or("app"));
    let files = if !targets.is_empty() {
        let from_root = expand_globs(&root, targets)?;
        if !from_root.is_empty() {
            from_root
        } else {
            expand_globs(&frontend_root, targets)?
        }
    } else {
        expand_globs(&frontend_root, targets)?
    };

    let mut violations = Vec::new();
    for file in &files {
        let analysis = analyze_file(file, &root)?;
        for facts in &analysis.components {
            if effective_no_fetch && !facts.fetches.is_empty() {
                violations.push(Violation {
                    component: facts.name.clone(),
                    file: facts.file.clone(),
                    rule: "assert-no-fetch".to_string(),
                    detail: facts
                        .fetches
                        .first()
                        .map(|f| f.shape.clone().unwrap_or_default()),
                });
            }
        }
    }
    Ok(violations)
}
