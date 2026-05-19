use super::{ast, runtime_deps, RULE_ID};
use crate::codebase::dependencies::graph::DepGraph;
use crate::codebase::rules::RuleFinding;
use crate::codebase::ts_resolver::ImportResolver;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub(super) struct DynamicCheckContext<'a> {
    pub(super) root: &'a Path,
    pub(super) file: &'a Path,
    pub(super) resolver: &'a ImportResolver<'a>,
    pub(super) graph: &'a DepGraph,
    pub(super) mocks: &'a HashSet<PathBuf>,
    pub(super) dependency_cache: &'a Mutex<HashMap<PathBuf, Arc<Vec<PathBuf>>>>,
    pub(super) findings: &'a mut Vec<RuleFinding>,
}

pub(super) fn check_dynamic_import(ctx: &mut DynamicCheckContext<'_>, import: ast::DynamicImport) {
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
    let cache_hit = ctx.dependency_cache.lock().unwrap().get(&target).map(Arc::clone);
    let deps = match cache_hit {
        Some(d) => d,
        None => {
            let new_deps = Arc::new(runtime_deps(ctx.graph, target.clone()));
            ctx.dependency_cache
                .lock()
                .unwrap()
                .insert(target.clone(), Arc::clone(&new_deps));
            new_deps
        }
    };
    for dependency in std::iter::once(target).chain(deps.iter().cloned()) {
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
