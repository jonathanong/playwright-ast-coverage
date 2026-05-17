mod ast;
mod checker;
mod config;
mod manual_mocks;
mod reachable;
mod runtime;

use super::RuleFinding;
use crate::codebase::dependencies::graph::{DepGraph, GraphBuildPlan};
use crate::codebase::rules::test_no_unmocked_dynamic_imports::checker::{
    check_dynamic_import, DynamicCheckContext,
};
use crate::codebase::ts_resolver::{load_tsconfig, normalize_path, ImportResolver, TsConfig};
use crate::codebase::ts_source::{discover_files, has_disable_comment, has_disable_file_comment};
use crate::config::v2::NoMistakesConfig;
use anyhow::{Context, Result};
use runtime::runtime_deps;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub const RULE_ID: &str = "test-no-unmocked-dynamic-imports";

pub fn check(
    root: &Path,
    config: &NoMistakesConfig,
    tsconfig_path: Option<&Path>,
) -> Result<Vec<RuleFinding>> {
    let files = discover_files(root, &config.filesystem.skip_directories);
    let tsconfig = resolve_tsconfig(root, tsconfig_path)?;
    let resolver = ImportResolver::new(&tsconfig);
    let graph = DepGraph::build_with_plan(root, &tsconfig, GraphBuildPlan::all())?;
    let manual_mocks = manual_mocks::discover(root, &config.filesystem.skip_directories);
    let mut dependency_cache = HashMap::new();
    let mut findings = Vec::new();

    for file in matching_test_files(root, &files, config)? {
        let source = std::fs::read_to_string(&file)
            .context(format!("failed to read test file {}", file.display()))?;
        if has_disable_file_comment(&source, RULE_ID) {
            continue;
        }
        let facts = ast::extract(&file, &source)?;
        let mut mocks = manual_mocks.clone();
        mocks.extend(setup_mocks(root, config, &file, &resolver)?);
        mocks.extend(resolve_mock_specifiers(
            &facts.mock_specifiers,
            &file,
            &resolver,
        ));
        let mut check_context = DynamicCheckContext {
            root,
            file: &file,
            resolver: &resolver,
            graph: &graph,
            mocks: &mocks,
            dependency_cache: &mut dependency_cache,
            findings: &mut findings,
        };
        for import in facts.dynamic_imports {
            if has_disable_comment(&source, import.line as u32, RULE_ID) {
                continue;
            }
            check_dynamic_import(&mut check_context, import);
        }
        let reachable_context = reachable::ReachableContext {
            root,
            config,
            resolver: &resolver,
            graph: &graph,
        };
        let reachable_result = reachable::check(
            reachable_context,
            &file,
            &mocks,
            &mut dependency_cache,
            &mut findings,
        );
        reachable_result?;
    }

    findings.sort_by_key(|f| (f.file.clone(), f.line, f.target.clone()));
    Ok(findings)
}

fn resolve_tsconfig(root: &Path, tsconfig_path: Option<&Path>) -> Result<TsConfig> {
    match tsconfig_path {
        Some(path) => load_tsconfig(path),
        None => match crate::codebase::ts_resolver::find_tsconfig(root) {
            Some(path) => load_tsconfig(&path),
            None => Ok(TsConfig {
                dir: root.to_path_buf(),
                paths: vec![],
                paths_dir: root.to_path_buf(),
                base_url: None,
            }),
        },
    }
}

fn resolve_mock_specifiers(
    specifiers: &[String],
    file: &Path,
    resolver: &ImportResolver<'_>,
) -> HashSet<PathBuf> {
    specifiers
        .iter()
        .map(|specifier| {
            resolver
                .resolve(specifier, file)
                .unwrap_or_else(|| PathBuf::from(specifier))
        })
        .collect()
}

fn setup_mocks(
    root: &Path,
    config: &NoMistakesConfig,
    test_file: &Path,
    resolver: &ImportResolver<'_>,
) -> Result<HashSet<PathBuf>> {
    let mut mocks = HashSet::new();
    let rel_path = crate::codebase::ts_source::relative_slash_path(root, test_file);
    for setup in config::setup_files_for_test(root, config, rel_path)? {
        let source = std::fs::read_to_string(&setup)
            .context(format!("failed to read setup file {}", setup.display()))?;
        let facts = ast::extract(&setup, &source)?;
        mocks.extend(resolve_mock_specifiers(
            &facts.mock_specifiers,
            &setup,
            resolver,
        ));
    }
    Ok(mocks)
}

fn matching_test_files(
    root: &Path,
    files: &[PathBuf],
    config: &NoMistakesConfig,
) -> Result<Vec<PathBuf>> {
    let filter = config::test_filter(root, config)?;
    Ok(files
        .iter()
        .filter(|file| crate::codebase::dependencies::extract::is_indexable(file))
        .filter(|file| filter.is_match(crate::codebase::ts_source::relative_slash_path(root, file)))
        .map(|file| normalize_path(file))
        .collect())
}

#[cfg(test)]
mod tests;
