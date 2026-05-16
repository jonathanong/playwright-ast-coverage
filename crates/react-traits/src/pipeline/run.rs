use crate::analyze::file::analyze_file;
use crate::cli::Cli;
use crate::pipeline::glob::expand_globs;
use crate::report::types::{AggregatedFacts, ComponentFacts, FileConfig, RootConfig};
use anyhow::Result;
use no_mistakes_core::config;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub(crate) fn run_analyze(
    base_root: &Path,
    cli: &Cli,
    targets: &[String],
    _depth: Option<usize>,
) -> Result<Vec<ComponentFacts>> {
    let (root, file_config) = load_root_and_config(base_root, cli)?;
    let frontend_root = root.join(file_config.frontend_root.as_deref().unwrap_or("app"));
    // Try glob from root first (targets like "app/components/*.tsx"),
    // fall back to frontend_root (targets like "components/*.tsx").
    let files = if !targets.is_empty() {
        let from_root = expand_globs(&root, targets)?;
        if !from_root.is_empty() {
            from_root
        } else {
            expand_globs(&frontend_root, targets)
                .expect("same patterns already validated; infallible")
        }
    } else {
        expand_globs(&frontend_root, targets).expect("empty patterns always succeed")
    };

    let mut results = Vec::new();
    let mut file_cache: HashMap<PathBuf, Vec<ComponentFacts>> = HashMap::new();

    for file in &files {
        let analysis = analyze_file(file, &root)?;
        file_cache.insert(file.clone(), analysis.components.clone());
        results.extend(analysis.components);
    }

    let results = results
        .into_iter()
        .map(|mut facts| {
            let agg = aggregate_children(&facts, &file_cache, &root, &mut HashSet::new());
            if agg != AggregatedFacts::default() {
                facts.inherited_from_children = Some(agg);
            }
            facts
        })
        .collect();

    Ok(results)
}

fn aggregate_children(
    facts: &ComponentFacts,
    file_cache: &HashMap<PathBuf, Vec<ComponentFacts>>,
    root: &Path,
    visited: &mut HashSet<String>,
) -> AggregatedFacts {
    let mut agg = AggregatedFacts::default();
    for child_ref in &facts.children {
        let key = format!("{}#{}", child_ref.file, child_ref.name);
        if visited.contains(&key) {
            continue;
        }
        visited.insert(key.clone());
        let child_path = root.join(&child_ref.file);
        let children = file_cache.get(&child_path).or_else(|| {
            child_path
                .canonicalize()
                .ok()
                .and_then(|p| file_cache.get(&p))
        });
        if let Some(components) = children {
            for child_facts in components {
                if child_facts.name == child_ref.name {
                    agg.has_state |= child_facts.has_state;
                    agg.has_props |= child_facts.has_props;
                    agg.passes_props |= child_facts.passes_props;
                    agg.uses_memo |= child_facts.uses_memo;
                    agg.uses_context_provider |= child_facts.uses_context_provider;
                    agg.uses_suspense |= child_facts.uses_suspense;
                    agg.has_fetch |= !child_facts.fetches.is_empty();
                    let child_agg = aggregate_children(child_facts, file_cache, root, visited);
                    agg.has_state |= child_agg.has_state;
                    agg.has_fetch |= child_agg.has_fetch;
                    agg.uses_suspense |= child_agg.uses_suspense;
                    agg.uses_context_provider |= child_agg.uses_context_provider;
                    agg.uses_memo |= child_agg.uses_memo;
                    agg.has_props |= child_agg.has_props;
                    agg.passes_props |= child_agg.passes_props;
                }
            }
        }
    }
    agg
}

#[cfg(test)]
mod tests;

pub(crate) fn load_root_and_config(base_root: &Path, cli: &Cli) -> Result<(PathBuf, FileConfig)> {
    let root = base_root.join(&cli.root);
    let stems = [".no-mistakes", ".react-traits"];
    let root_config: RootConfig = config::load_config(&root, cli.config.as_deref(), &stems)?;
    let mut file_config = root_config.legacy;
    if let Some(overrides) = root_config.react_traits {
        if overrides.frontend_root.is_some() {
            file_config.frontend_root = overrides.frontend_root;
        }
        if overrides.assert_no_fetch.is_some() {
            file_config.assert_no_fetch = overrides.assert_no_fetch;
        }
    }
    Ok((root, file_config))
}
