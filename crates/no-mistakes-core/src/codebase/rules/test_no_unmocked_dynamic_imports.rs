mod ast;
mod config;
mod manual_mocks;
mod runtime;

use super::RuleFinding;
use crate::codebase::dependencies::graph::{DepGraph, GraphBuildPlan};
use crate::codebase::ts_resolver::{load_tsconfig, normalize_path, ImportResolver, TsConfig};
use crate::codebase::ts_source::{discover_files, has_disable_comment, has_disable_file_comment};
use crate::config::v2::NoMistakesConfig;
use anyhow::Result;
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
        let source = std::fs::read_to_string(&file).unwrap_or_default();
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
    }

    findings.sort_by_key(|f| (f.file.clone(), f.line, f.target.clone()));
    Ok(findings)
}

struct DynamicCheckContext<'a> {
    root: &'a Path,
    file: &'a Path,
    resolver: &'a ImportResolver<'a>,
    graph: &'a DepGraph,
    mocks: &'a HashSet<PathBuf>,
    dependency_cache: &'a mut HashMap<PathBuf, Vec<PathBuf>>,
    findings: &'a mut Vec<RuleFinding>,
}

fn check_dynamic_import(ctx: &mut DynamicCheckContext<'_>, import: ast::DynamicImport) {
    let Some(specifier) = import.specifier else {
        push_finding(ctx.root, ctx.file, import.line, None, None, ctx.findings);
        return;
    };
    let Some(target) = ctx.resolver.resolve(&specifier, ctx.file) else {
        if !ctx.mocks.contains(&PathBuf::from(&specifier)) {
            push_finding(
                ctx.root,
                ctx.file,
                import.line,
                Some(specifier),
                None,
                ctx.findings,
            );
        }
        return;
    };
    if ctx.mocks.contains(&target) {
        return;
    }
    let mut required = vec![target.clone()];
    let dependencies = if let Some(dependencies) = ctx.dependency_cache.get(&target) {
        dependencies.clone()
    } else {
        let dependencies = runtime_deps(ctx.graph, target.clone());
        ctx.dependency_cache.insert(target, dependencies.clone());
        dependencies
    };
    required.extend(dependencies);
    for dependency in required {
        if !ctx.mocks.contains(&dependency) {
            push_finding(
                ctx.root,
                ctx.file,
                import.line,
                Some(specifier.clone()),
                Some(dependency),
                ctx.findings,
            );
        }
    }
}

fn push_finding(
    root: &Path,
    file: &Path,
    line: usize,
    specifier: Option<String>,
    target: Option<PathBuf>,
    findings: &mut Vec<RuleFinding>,
) {
    let rel_file = crate::codebase::ts_source::relative_slash_path(root, file);
    let rel_target = target
        .as_ref()
        .map(|path| crate::codebase::ts_source::relative_slash_path(root, path));
    let label = rel_target
        .as_deref()
        .or(specifier.as_deref())
        .unwrap_or("dynamic import")
        .to_string();
    findings.push(RuleFinding {
        rule: RULE_ID.to_string(),
        file: rel_file,
        line,
        import: specifier,
        target: rel_target,
        message: format!("dynamic import dependency `{label}` must be mocked"),
    });
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
        let source = std::fs::read_to_string(&setup).unwrap_or_default();
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
