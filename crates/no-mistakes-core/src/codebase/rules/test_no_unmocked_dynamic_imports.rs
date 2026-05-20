pub(crate) mod ast;
mod checker;
pub(crate) mod config;
mod manual_mocks;
mod reachable;
mod runtime;
mod with_facts;

use super::RuleFinding;
use crate::codebase::dependencies::graph::{DepGraph, GraphBuildPlan};
use crate::codebase::rules::test_no_unmocked_dynamic_imports::checker::{
    check_dynamic_import, DynamicCheckContext,
};
use crate::codebase::ts_resolver::{load_tsconfig, normalize_path, ImportResolver, TsConfig};
use crate::codebase::ts_source::{discover_files, has_disable_comment, has_disable_file_comment};
use crate::config::v2::NoMistakesConfig;
use anyhow::{Context, Result};
use dashmap::DashMap;
use runtime::runtime_deps;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
pub use with_facts::check_with_facts;

pub const RULE_ID: &str = "test-no-unmocked-dynamic-imports";

pub fn check(
    root: &Path,
    config: &NoMistakesConfig,
    tsconfig_path: Option<&Path>,
) -> Result<Vec<RuleFinding>> {
    let files = discover_files(root, &config.filesystem.skip_directories);
    let tsconfig = resolve_tsconfig(root, tsconfig_path)?;
    let graph =
        DepGraph::build_with_plan(root, &tsconfig, GraphBuildPlan::imports_and_workspace())?;
    let manual_mocks = manual_mocks::discover(root, &config.filesystem.skip_directories);
    check_inner(root, config, &files, &tsconfig, &graph, &manual_mocks)
}

pub(super) fn check_inner(
    root: &Path,
    config: &NoMistakesConfig,
    files: &[PathBuf],
    tsconfig: &TsConfig,
    graph: &DepGraph,
    manual_mocks: &HashSet<PathBuf>,
) -> Result<Vec<RuleFinding>> {
    let resolver = ImportResolver::new(tsconfig);
    let dependency_cache: DashMap<PathBuf, Arc<Vec<PathBuf>>> = DashMap::new();
    let file_cache: DashMap<PathBuf, Arc<reachable::CachedFileFacts>> = DashMap::new();
    let mut findings = Vec::new();
    let setup_data = config::precompute_setup_data(root, config)?;
    let test_files = matching_test_files(root, files, config)?;
    let setup_mock_map = precompute_setup_mock_map(root, &test_files, &setup_data, &resolver)?;

    for file in test_files {
        let source = std::fs::read_to_string(&file)
            .context(format!("failed to read test file {}", file.display()))?;
        if has_disable_file_comment(&source, RULE_ID) {
            continue;
        }
        let facts = ast::extract(&file, &source)?;
        let mut mocks = manual_mocks.clone();
        mocks.extend(setup_mocks(root, &setup_data, &file, &setup_mock_map));
        mocks.extend(resolve_mock_specifiers(
            &facts.mock_specifiers,
            &file,
            &resolver,
        ));
        let mut check_context = DynamicCheckContext {
            root,
            file: &file,
            resolver: &resolver,
            graph,
            mocks: &mocks,
            dependency_cache: &dependency_cache,
            findings: &mut findings,
        };
        for import in facts.dynamic_imports {
            if has_disable_comment(&source, import.line as u32, RULE_ID) {
                continue;
            }
            check_dynamic_import(&mut check_context, import);
        }
        reachable::check(
            reachable::ReachableContext {
                root,
                config,
                resolver: &resolver,
                graph,
                shared: None,
                file_cache: Some(&file_cache),
            },
            &file,
            &mocks,
            &dependency_cache,
            &mut findings,
        )?;
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

fn precompute_setup_mock_map(
    root: &Path,
    test_files: &[PathBuf],
    setup_data: &[config::ConfigSetupData],
    resolver: &ImportResolver<'_>,
) -> Result<HashMap<PathBuf, HashSet<PathBuf>>> {
    let unique_setups: HashSet<PathBuf> = test_files
        .iter()
        .flat_map(|f| {
            let rel = crate::codebase::ts_source::relative_slash_path(root, f);
            config::setup_files_for_test_precomputed(&rel, setup_data)
        })
        .collect();
    unique_setups
        .into_iter()
        .map(|setup| {
            let source = std::fs::read_to_string(&setup)
                .context(format!("failed to read setup file {}", setup.display()))?;
            let facts = ast::extract(&setup, &source)?;
            Ok((
                setup.clone(),
                resolve_mock_specifiers(&facts.mock_specifiers, &setup, resolver),
            ))
        })
        .collect()
}

fn setup_mocks(
    root: &Path,
    setup_data: &[config::ConfigSetupData],
    test_file: &Path,
    mock_map: &HashMap<PathBuf, HashSet<PathBuf>>,
) -> HashSet<PathBuf> {
    let rel_path = crate::codebase::ts_source::relative_slash_path(root, test_file);
    let mut mocks = HashSet::new();
    for setup in config::setup_files_for_test_precomputed(&rel_path, setup_data) {
        if let Some(m) = mock_map.get(&setup) {
            mocks.extend(m.iter().cloned());
        }
    }
    mocks
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
        .filter(|file| {
            filter.is_match(&crate::codebase::ts_source::relative_slash_path(root, file))
        })
        .map(|file| normalize_path(file))
        .collect())
}

#[cfg(test)]
mod tests;
