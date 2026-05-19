use super::checker::{check_dynamic_import, DynamicCheckContext};
use super::{ast, runtime_deps, RULE_ID};
use crate::codebase::check_facts::CheckFactMap;
use crate::codebase::dependencies::graph::DepGraph;
use crate::codebase::rules::RuleFinding;
use crate::codebase::ts_resolver::ImportResolver;
use crate::codebase::ts_source::{has_disable_comment, has_disable_file_comment};
use crate::config::v2::NoMistakesConfig;
use anyhow::{Context, Result};
use dashmap::DashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(super) struct CachedFileFacts {
    pub(super) source: String,
    pub(super) dynamic_imports: Vec<ast::DynamicImport>,
}

pub(super) fn check(
    ctx: ReachableContext<'_>,
    test_file: &Path,
    mocks: &HashSet<PathBuf>,
    dependency_cache: &DashMap<PathBuf, Arc<Vec<PathBuf>>>,
    findings: &mut Vec<RuleFinding>,
) -> Result<()> {
    let test_reachable = dependency_cache
        .entry(test_file.to_path_buf())
        .or_insert_with(|| Arc::new(runtime_deps(ctx.graph, test_file.to_path_buf())))
        .clone();
    for file in test_reachable.iter() {
        if !crate::codebase::dependencies::extract::is_indexable(file)
            || is_under_skipped_dir(ctx.root, ctx.config, file)
        {
            continue;
        }
        if let Some(shared) = ctx.shared {
            if let Some(file_facts) = shared.ts.get(file) {
                if file_facts.parse_error.is_some() {
                    continue;
                }
                if let (Some(source), Some(facts)) = (
                    file_facts.source.as_deref(),
                    file_facts.dynamic_imports.as_ref(),
                ) {
                    if has_disable_file_comment(source, RULE_ID) {
                        continue;
                    }
                    let mut check_context = DynamicCheckContext {
                        root: ctx.root,
                        file,
                        resolver: ctx.resolver,
                        graph: ctx.graph,
                        mocks,
                        dependency_cache,
                        findings,
                    };
                    for import in &facts.dynamic_imports {
                        if !has_disable_comment(source, import.line as u32, RULE_ID) {
                            check_dynamic_import(&mut check_context, import.clone());
                        }
                    }
                    continue;
                }
            }
        }
        let cached = get_or_cache_file(file, ctx.file_cache)?;
        if has_disable_file_comment(&cached.source, RULE_ID) {
            continue;
        }
        let mut check_context = DynamicCheckContext {
            root: ctx.root,
            file,
            resolver: ctx.resolver,
            graph: ctx.graph,
            mocks,
            dependency_cache,
            findings,
        };
        for import in &cached.dynamic_imports {
            if has_disable_comment(&cached.source, import.line as u32, RULE_ID) {
                continue;
            }
            check_dynamic_import(&mut check_context, import.clone());
        }
    }
    Ok(())
}

pub(super) struct ReachableContext<'a> {
    pub(super) root: &'a Path,
    pub(super) config: &'a NoMistakesConfig,
    pub(super) resolver: &'a ImportResolver<'a>,
    pub(super) graph: &'a DepGraph,
    pub(super) shared: Option<&'a CheckFactMap>,
    pub(super) file_cache: Option<&'a DashMap<PathBuf, Arc<CachedFileFacts>>>,
}

fn get_or_cache_file(
    file: &PathBuf,
    cache: Option<&DashMap<PathBuf, Arc<CachedFileFacts>>>,
) -> Result<Arc<CachedFileFacts>> {
    if let Some(cache) = cache {
        if let Some(cached) = cache.get(file) {
            return Ok(cached.clone());
        }
        let source = std::fs::read_to_string(file)
            .context(format!("failed to read dependency file {}", file.display()))?;
        let facts = ast::extract(file, &source)?;
        let arc = Arc::new(CachedFileFacts {
            source,
            dynamic_imports: facts.dynamic_imports,
        });
        cache.insert(file.clone(), arc.clone());
        return Ok(arc);
    }
    let source = std::fs::read_to_string(file)
        .context(format!("failed to read dependency file {}", file.display()))?;
    let facts = ast::extract(file, &source)?;
    Ok(Arc::new(CachedFileFacts {
        source,
        dynamic_imports: facts.dynamic_imports,
    }))
}

fn is_under_skipped_dir(root: &Path, config: &NoMistakesConfig, file: &Path) -> bool {
    file.strip_prefix(root).ok().is_some_and(|rel| {
        if config
            .filesystem
            .skip_directories
            .iter()
            .map(Path::new)
            .any(|skip| rel == skip || rel.starts_with(skip))
        {
            return true;
        }
        rel.components().any(|component| {
            component
                .as_os_str()
                .to_str()
                .is_some_and(|name| crate::codebase::ts_source::SKIP_DIRS.contains(&name))
        })
    })
}
