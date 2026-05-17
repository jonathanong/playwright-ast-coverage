pub(crate) use no_mistakes_core::fetch::route_analysis::collect_route_fetches;

use crate::analyze::routes::route_reaches_target;
use crate::pipeline::cache::Cache;
use crate::pipeline::target::{route_matches_target, TargetSpec};
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(crate) fn check_route_matches(
    route: &no_mistakes_core::routes::Route,
    target_specs: &[TargetSpec],
    wrapper_files: &[PathBuf],
    cache: &mut Cache,
) -> Result<(bool, Vec<String>)> {
    let mut newly_matched = Vec::new();

    if target_specs.is_empty() {
        return Ok((true, newly_matched));
    }

    let mut matched = false;
    'target_match: for target in target_specs {
        if route_matches_target(&route.pattern, &target.raw) {
            matched = true;
            newly_matched.push(target.raw.clone());
            continue;
        }

        if let Some(target_file) = &target.file {
            let reaches_route_target = reaches_target(&route.file, target_file, cache)?;
            if reaches_route_target {
                matched = true;
                newly_matched.push(target.raw.clone());
                continue 'target_match;
            }

            let mut wrapper_file_matches = false;
            for wrapper_file in wrapper_files {
                if wrapper_file == target_file {
                    wrapper_file_matches = true;
                    break;
                }

                let reaches_wrapper_target = reaches_target(wrapper_file, target_file, cache)?;
                if reaches_wrapper_target {
                    wrapper_file_matches = true;
                    break;
                }
            }

            if wrapper_file_matches {
                matched = true;
                newly_matched.push(target.raw.clone());
                continue 'target_match;
            }
        }
    }

    Ok((matched, newly_matched))
}

fn reaches_target(source_file: &Path, target_file: &Path, cache: &mut Cache) -> Result<bool> {
    let mut visited_targets = HashSet::new();
    route_reaches_target(
        source_file,
        target_file,
        &mut visited_targets,
        &mut cache.imports,
    )
}
