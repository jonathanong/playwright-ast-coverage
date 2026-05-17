use crate::react_traits::analyze::file::analyze_file;
use crate::react_traits::pipeline::glob::expand_globs;
use crate::react_traits::report::types::{AggregatedFacts, ComponentFacts, FileConfig, RootConfig};
use anyhow::Result;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub fn run_analyze(
    root: &Path,
    config_path: Option<&Path>,
    targets: &[String],
    depth: Option<usize>,
) -> Result<Vec<ComponentFacts>> {
    let stems = [".no-mistakes", ".react-traits"];
    let root_config: RootConfig = crate::config::load_config(root, config_path, &stems)?;
    let file_config = root_config.into_file_config();
    run_analyze_inner(root, &file_config, targets, depth)
}

pub(crate) fn run_analyze_inner(
    root: &Path,
    file_config: &FileConfig,
    targets: &[String],
    _depth: Option<usize>,
) -> Result<Vec<ComponentFacts>> {
    let frontend_root = root.join(file_config.frontend_root.as_deref().unwrap_or("app"));
    // Expand globs from root first; only fall back to frontend_root when root yields no matches.
    // Skip the frontend_root.exists() gate entirely when patterns match at root level.
    let files = if !targets.is_empty() {
        let from_root = expand_globs(root, targets)?;
        if !from_root.is_empty() {
            from_root
        } else {
            if !frontend_root.exists() {
                anyhow::bail!("frontend root not found: {}", frontend_root.display());
            }
            expand_globs(&frontend_root, targets)?
        }
    } else {
        if !frontend_root.exists() {
            anyhow::bail!("frontend root not found: {}", frontend_root.display());
        }
        expand_globs(&frontend_root, targets)?
    };

    let mut file_cache: HashMap<PathBuf, Vec<ComponentFacts>> = HashMap::new();
    let analyses = files
        .par_iter()
        .map(|file| analyze_file(file, root).map(|analysis| (file.clone(), analysis.components)))
        .collect::<Result<Vec<_>>>()?;

    let mut results = Vec::new();
    for (file, components) in analyses {
        file_cache.insert(file, components.clone());
        results.extend(components);
    }

    let mut all_results = Vec::new();
    for mut facts in results {
        let agg = aggregate_children(&facts, &mut file_cache, root, &mut HashSet::new());
        if agg != AggregatedFacts::default() {
            facts.inherited_from_children = Some(agg);
        }
        all_results.push(facts);
    }

    Ok(all_results)
}

fn aggregate_children(
    facts: &ComponentFacts,
    file_cache: &mut HashMap<PathBuf, Vec<ComponentFacts>>,
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
        // Analyze on-demand and cache so repeated child refs avoid redundant parsing (Cgv-B).
        if !file_cache.contains_key(&child_path) {
            match analyze_file(&child_path, root) {
                Ok(a) => {
                    file_cache.insert(child_path.clone(), a.components);
                }
                Err(_) => continue,
            }
        }
        // Clone only the matching component (not the whole Vec) so the borrow of
        // file_cache is dropped before the recursive mutable borrow in aggregate_children.
        let child_facts_opt = file_cache
            .get(&child_path)
            .and_then(|comps| comps.iter().find(|c| c.name == child_ref.name))
            .cloned();
        if let Some(child_facts) = child_facts_opt {
            agg.has_state |= child_facts.has_state;
            agg.has_props |= child_facts.has_props;
            agg.passes_props |= child_facts.passes_props;
            agg.uses_memo |= child_facts.uses_memo;
            agg.uses_context_provider |= child_facts.uses_context_provider;
            agg.uses_suspense |= child_facts.uses_suspense;
            agg.has_fetch |= !child_facts.fetches.is_empty();
            let child_agg = aggregate_children(&child_facts, file_cache, root, visited);
            agg.has_state |= child_agg.has_state;
            agg.has_fetch |= child_agg.has_fetch;
            agg.uses_suspense |= child_agg.uses_suspense;
            agg.uses_context_provider |= child_agg.uses_context_provider;
            agg.uses_memo |= child_agg.uses_memo;
            agg.has_props |= child_agg.has_props;
            agg.passes_props |= child_agg.passes_props;
        }
    }
    agg
}
