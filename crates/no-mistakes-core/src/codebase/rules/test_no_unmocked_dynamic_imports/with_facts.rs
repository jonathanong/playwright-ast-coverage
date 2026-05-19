use super::checker::{check_dynamic_import, DynamicCheckContext};
use super::{config, manual_mocks, matching_test_files, reachable, resolve_mock_specifiers};
use super::{resolve_tsconfig, RuleFinding, RULE_ID};
use crate::codebase::check_facts::CheckFactMap;
use crate::codebase::dependencies::graph::{DepGraph, GraphBuildPlan};
use crate::codebase::rules::test_no_unmocked_dynamic_imports::runtime::runtime_deps;
use crate::codebase::ts_resolver::ImportResolver;
use crate::codebase::ts_source::{has_disable_comment, has_disable_file_comment};
use crate::config::v2::NoMistakesConfig;
use anyhow::Result;
use dashmap::DashMap;
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn check_with_facts(
    root: &Path,
    config: &NoMistakesConfig,
    tsconfig_path: Option<&Path>,
    shared: &CheckFactMap,
) -> Result<Vec<RuleFinding>> {
    let files = shared.files().to_vec();
    let tsconfig = resolve_tsconfig(root, tsconfig_path)?;
    let resolver = ImportResolver::new(&tsconfig);
    let ts_facts = shared.ts_facts();
    let graph = DepGraph::build_with_plan_file_list_and_facts(
        root,
        &tsconfig,
        GraphBuildPlan::imports_and_workspace(),
        files.clone(),
        &ts_facts,
    );
    let manual_mocks = manual_mocks::discover(root, &config.filesystem.skip_directories);
    let test_files = matching_test_files(root, &files, config)?;
    let setup_data = config::precompute_setup_data(root, config)?;

    // Pre-populate the dependency cache for all test files in parallel so that
    // `reachable::check` hits the cache instead of re-running BFS per test.
    let dependency_cache: DashMap<PathBuf, Arc<Vec<PathBuf>>> = DashMap::new();
    test_files.par_iter().for_each(|file| {
        let deps = Arc::new(runtime_deps(&graph, file.clone()));
        dependency_cache.entry(file.clone()).or_insert(deps);
    });

    let per_test: Vec<Vec<RuleFinding>> = test_files
        .into_par_iter()
        .map(|file| {
            let Some(file_facts) = shared.ts.get(&file) else {
                anyhow::bail!("missing shared facts for {}", file.display());
            };
            let Some(source) = file_facts.source.as_deref() else {
                anyhow::bail!("missing source facts for {}", file.display());
            };
            if has_disable_file_comment(source, RULE_ID) {
                return Ok(Vec::new());
            }
            if let Some(error) = &file_facts.parse_error {
                anyhow::bail!("failed to parse {}: {error}", file.display());
            }
            let Some(facts) = file_facts.dynamic_imports.as_ref() else {
                anyhow::bail!("missing dynamic import facts for {}", file.display());
            };
            let mut mocks = manual_mocks.clone();
            mocks.extend(setup_mocks_with_facts(
                root, &setup_data, &file, &resolver, shared,
            )?);
            mocks.extend(resolve_mock_specifiers(
                &facts.mock_specifiers,
                &file,
                &resolver,
            ));
            let mut local_findings = Vec::new();
            {
                let mut check_context = DynamicCheckContext {
                    root,
                    file: &file,
                    resolver: &resolver,
                    graph: &graph,
                    mocks: &mocks,
                    dependency_cache: &dependency_cache,
                    findings: &mut local_findings,
                };
                for import in &facts.dynamic_imports {
                    if !has_disable_comment(source, import.line as u32, RULE_ID) {
                        check_dynamic_import(&mut check_context, import.clone());
                    }
                }
            }
            reachable::check(
                reachable::ReachableContext {
                    root,
                    config,
                    resolver: &resolver,
                    graph: &graph,
                    shared: Some(shared),
                },
                &file,
                &mocks,
                &dependency_cache,
                &mut local_findings,
            )?;
            Ok(local_findings)
        })
        .collect::<Result<Vec<_>>>()?;

    let mut findings: Vec<RuleFinding> = per_test.into_iter().flatten().collect();
    findings.sort_by_key(|f| (f.file.clone(), f.line, f.target.clone()));
    Ok(findings)
}

fn setup_mocks_with_facts(
    root: &Path,
    setup_data: &[config::ConfigSetupData],
    test_file: &Path,
    resolver: &ImportResolver<'_>,
    shared: &CheckFactMap,
) -> Result<HashSet<PathBuf>> {
    let mut mocks = HashSet::new();
    let rel_path = crate::codebase::ts_source::relative_slash_path(root, test_file);
    for setup in config::setup_files_for_test_precomputed(&rel_path, setup_data) {
        let Some(file_facts) = shared.ts.get(&setup) else {
            anyhow::bail!("missing shared facts for {}", setup.display());
        };
        if let Some(error) = &file_facts.parse_error {
            anyhow::bail!("failed to parse {}: {error}", setup.display());
        }
        let Some(facts) = file_facts.dynamic_imports.as_ref() else {
            anyhow::bail!("missing dynamic import facts for {}", setup.display());
        };
        mocks.extend(resolve_mock_specifiers(
            &facts.mock_specifiers,
            &setup,
            resolver,
        ));
    }
    Ok(mocks)
}
