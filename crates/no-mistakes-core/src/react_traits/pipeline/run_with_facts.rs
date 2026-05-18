use crate::react_traits::pipeline::glob::expand_globs;
use crate::react_traits::report::types::{AggregatedFacts, ComponentFacts, FileConfig};
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub(crate) fn run_analyze_inner_with_facts(
    root: &Path,
    file_config: &FileConfig,
    targets: &[String],
    shared: &crate::codebase::check_facts::CheckFactMap,
) -> Result<Vec<ComponentFacts>> {
    let files = target_files(root, file_config, targets)?;
    let file_cache = shared
        .ts
        .iter()
        .filter_map(|(path, facts)| {
            facts
                .react
                .as_ref()
                .map(|analysis| (path.clone(), analysis.components.clone()))
        })
        .collect::<HashMap<_, _>>();
    let child_path_index = child_path_index(root, &file_cache);
    let mut all_results = Vec::new();
    for file in files {
        if let Some(error) = shared
            .ts
            .get(&file)
            .and_then(|facts| facts.parse_error.as_ref())
        {
            anyhow::bail!("failed to parse {}: {error}", file.display());
        }
        let Some(components) = file_cache.get(&file).cloned() else {
            continue;
        };
        for mut facts in components {
            let agg = aggregate_children_cached(
                &facts,
                &file_cache,
                &child_path_index,
                &mut HashSet::new(),
            );
            if agg != AggregatedFacts::default() {
                facts.inherited_from_children = Some(agg);
            }
            all_results.push(facts);
        }
    }
    Ok(all_results)
}

fn target_files(root: &Path, file_config: &FileConfig, targets: &[String]) -> Result<Vec<PathBuf>> {
    let frontend_root = root.join(file_config.frontend_root.as_deref().unwrap_or("app"));
    let default_targets;
    let targets = if targets.is_empty() {
        default_targets = vec![
            "**/*.tsx".to_string(),
            "**/*.ts".to_string(),
            "**/*.jsx".to_string(),
            "**/*.js".to_string(),
        ];
        default_targets.as_slice()
    } else {
        targets
    };
    let from_root = expand_globs(root, targets)?;
    if !from_root.is_empty() {
        return Ok(from_root);
    }
    if !frontend_root.exists() {
        anyhow::bail!("frontend root not found: {}", frontend_root.display());
    }
    expand_globs(&frontend_root, targets)
}

fn aggregate_children_cached(
    facts: &ComponentFacts,
    file_cache: &HashMap<PathBuf, Vec<ComponentFacts>>,
    child_path_index: &HashMap<String, PathBuf>,
    visited: &mut HashSet<String>,
) -> AggregatedFacts {
    let mut agg = AggregatedFacts::default();
    for child_ref in &facts.children {
        let key = format!("{}#{}", child_ref.file, child_ref.name);
        if !visited.insert(key) {
            continue;
        }
        let child_facts_opt = child_path_index
            .get(&child_ref.file)
            .and_then(|path| file_cache.get(path))
            .and_then(|comps| comps.iter().find(|c| c.name == child_ref.name));
        if let Some(child_facts) = child_facts_opt {
            merge_component(&mut agg, child_facts);
            let child_agg =
                aggregate_children_cached(child_facts, file_cache, child_path_index, visited);
            merge_aggregate(&mut agg, &child_agg);
        }
    }
    agg
}

fn child_path_index(
    root: &Path,
    file_cache: &HashMap<PathBuf, Vec<ComponentFacts>>,
) -> HashMap<String, PathBuf> {
    let mut index = HashMap::new();
    for path in file_cache.keys() {
        index.insert(path.to_string_lossy().to_string(), path.clone());
        index.insert(
            crate::codebase::ts_source::relative_slash_path(root, path),
            path.clone(),
        );
    }
    index
}

fn merge_component(agg: &mut AggregatedFacts, facts: &ComponentFacts) {
    agg.has_state |= facts.has_state;
    agg.has_props |= facts.has_props;
    agg.passes_props |= facts.passes_props;
    agg.uses_memo |= facts.uses_memo;
    agg.uses_context_provider |= facts.uses_context_provider;
    agg.uses_suspense |= facts.uses_suspense;
    agg.has_fetch |= !facts.fetches.is_empty();
}

fn merge_aggregate(agg: &mut AggregatedFacts, child: &AggregatedFacts) {
    agg.has_state |= child.has_state;
    agg.has_fetch |= child.has_fetch;
    agg.uses_suspense |= child.uses_suspense;
    agg.uses_context_provider |= child.uses_context_provider;
    agg.uses_memo |= child.uses_memo;
    agg.has_props |= child.has_props;
    agg.passes_props |= child.passes_props;
}

#[cfg(test)]
mod tests;
