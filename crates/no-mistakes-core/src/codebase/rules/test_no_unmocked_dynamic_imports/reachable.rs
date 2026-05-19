use super::checker::{check_dynamic_import, DynamicCheckContext};
use super::{ast, runtime_deps, RULE_ID};
use crate::codebase::check_facts::CheckFactMap;
use crate::codebase::dependencies::graph::DepGraph;
use crate::codebase::rules::RuleFinding;
use crate::codebase::ts_resolver::ImportResolver;
use crate::codebase::ts_source::{has_disable_comment, has_disable_file_comment};
use crate::config::v2::NoMistakesConfig;
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub(super) fn check(
    ctx: ReachableContext<'_>,
    test_file: &Path,
    mocks: &HashSet<PathBuf>,
    dependency_cache: &Mutex<HashMap<PathBuf, Arc<Vec<PathBuf>>>>,
    findings: &mut Vec<RuleFinding>,
) -> Result<()> {
    for file in runtime_deps(ctx.graph, test_file.to_path_buf()) {
        if !crate::codebase::dependencies::extract::is_indexable(&file)
            || is_under_skipped_dir(ctx.root, ctx.config, &file)
        {
            continue;
        }
        if let Some(shared) = ctx.shared {
            if let Some(file_facts) = shared.ts.get(&file) {
                if file_facts.parse_error.is_some() {
                    continue;
                }
                let (Some(source), Some(facts)) =
                    (file_facts.source.as_deref(), file_facts.dynamic_imports.as_ref())
                else {
                    continue;
                };
                if has_disable_file_comment(source, RULE_ID) {
                    continue;
                }
                let mut check_context = DynamicCheckContext {
                    root: ctx.root,
                    file: &file,
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
        let source = std::fs::read_to_string(&file)
            .context(format!("failed to read dependency file {}", file.display()))?;
        if has_disable_file_comment(&source, RULE_ID) {
            continue;
        }
        let facts = ast::extract(&file, &source)?;
        let mut check_context = DynamicCheckContext {
            root: ctx.root,
            file: &file,
            resolver: ctx.resolver,
            graph: ctx.graph,
            mocks,
            dependency_cache,
            findings,
        };
        for import in facts.dynamic_imports {
            if has_disable_comment(&source, import.line as u32, RULE_ID) {
                continue;
            }
            check_dynamic_import(&mut check_context, import);
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
