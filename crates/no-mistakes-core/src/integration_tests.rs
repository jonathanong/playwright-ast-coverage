use crate::config::{v2::load_v2_config, CONFIG_EXTENSIONS};
use anyhow::Result;
use std::path::Path;

pub(crate) mod analysis;
mod calls;
mod config;
mod enforce;
mod project_config;
mod resolve;
mod test_config;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_errors;
#[cfg(test)]
mod tests_resolution;
#[cfg(test)]
mod tests_review;
pub(crate) mod types;

pub use types::IntegrationFinding;

pub fn check(root: &Path, config_path: Option<&Path>) -> Result<Vec<IntegrationFinding>> {
    if config_path.is_none() && !has_no_mistakes_config(root) {
        return Ok(Vec::new());
    }

    let config = load_v2_config(root, config_path)?;
    config::validate_config(&config)?;

    let suites = config::configured_suites(root, &config)?;
    if suites.is_empty() {
        return Ok(Vec::new());
    }

    let files = crate::codebase::ts_source::discover_source_files(
        root,
        &config.filesystem.skip_directories,
    );
    let tsconfig = project_config::resolve_tsconfig(root)?;
    let analyses = analysis::analyze_files(&files)?;
    let function_index = resolve::build_function_index(&analyses);
    let export_index = resolve::build_export_index(&analyses);
    let resolver = resolve::ImportResolution {
        analyses: &analyses,
        export_index: &export_index,
        tsconfig: &tsconfig,
    };

    let mut findings = Vec::new();
    for suite in &suites {
        let include = project_config::build_globset(&suite.include)?;
        let exclude = project_config::build_globset(&suite.exclude)?;
        for (file, file_analysis) in &analyses {
            let rel = crate::codebase::ts_source::relative_slash_path(root, file);
            if !include.is_match(&rel) || exclude.is_match(&rel) {
                continue;
            }
            for test in &file_analysis.tests {
                let integrations =
                    resolve::resolved_integrations(&test.function_key, &function_index, &resolver);
                findings.extend(enforce::enforce_policy(root, suite, test, &integrations));
            }
        }
    }
    sort_findings(&mut findings);
    Ok(findings)
}

pub fn check_with_facts(
    root: &Path,
    config_path: Option<&Path>,
    shared: &crate::codebase::check_facts::CheckFactMap,
) -> Result<Vec<IntegrationFinding>> {
    if config_path.is_none() && !has_no_mistakes_config(root) {
        return Ok(Vec::new());
    }

    let config = load_v2_config(root, config_path)?;
    config::validate_config(&config)?;

    let suites = config::configured_suites(root, &config)?;
    if suites.is_empty() {
        return Ok(Vec::new());
    }

    let tsconfig = project_config::resolve_tsconfig(root)?;
    fail_on_dropped_files(shared)?;
    let analyses = shared
        .ts
        .iter()
        .filter_map(|(path, facts)| {
            facts
                .integration
                .as_ref()
                .map(|analysis| (path.clone(), analysis.clone()))
        })
        .collect();
    check_suites(root, &suites, &tsconfig, &analyses)
}

fn fail_on_dropped_files(shared: &crate::codebase::check_facts::CheckFactMap) -> Result<()> {
    for (file, facts) in &shared.ts {
        if let Some(error) = &facts.parse_error {
            anyhow::bail!(
                "failed to parse integration file {}: {error}",
                file.display()
            );
        }
    }
    Ok(())
}

fn check_suites(
    root: &Path,
    suites: &[types::Suite],
    tsconfig: &crate::codebase::ts_resolver::TsConfig,
    analyses: &std::collections::BTreeMap<std::path::PathBuf, types::FileAnalysis>,
) -> Result<Vec<IntegrationFinding>> {
    let function_index = resolve::build_function_index(analyses);
    let export_index = resolve::build_export_index(analyses);
    let resolver = resolve::ImportResolution {
        analyses,
        export_index: &export_index,
        tsconfig,
    };

    let mut findings = Vec::new();
    for suite in suites {
        let include = project_config::build_globset(&suite.include)?;
        let exclude = project_config::build_globset(&suite.exclude)?;
        for (file, file_analysis) in analyses {
            let rel = crate::codebase::ts_source::relative_slash_path(root, file);
            if !include.is_match(&rel) || exclude.is_match(&rel) {
                continue;
            }
            for test in &file_analysis.tests {
                let integrations =
                    resolve::resolved_integrations(&test.function_key, &function_index, &resolver);
                findings.extend(enforce::enforce_policy(root, suite, test, &integrations));
            }
        }
    }
    sort_findings(&mut findings);
    Ok(findings)
}

fn has_no_mistakes_config(root: &Path) -> bool {
    CONFIG_EXTENSIONS
        .iter()
        .any(|extension| root.join(format!(".no-mistakes.{extension}")).exists())
}

fn sort_findings(findings: &mut Vec<IntegrationFinding>) {
    findings.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.framework.cmp(&b.framework))
            .then(a.suite.cmp(&b.suite))
    });
    findings.dedup_by(|a, b| {
        a.framework == b.framework
            && a.suite == b.suite
            && a.file == b.file
            && a.line == b.line
            && a.message == b.message
    });
}

fn tsconfig_without_config(root: &Path) -> crate::codebase::ts_resolver::TsConfig {
    crate::codebase::ts_resolver::TsConfig {
        dir: root.to_path_buf(),
        paths: Vec::new(),
        paths_dir: root.to_path_buf(),
        base_url: None,
    }
}
