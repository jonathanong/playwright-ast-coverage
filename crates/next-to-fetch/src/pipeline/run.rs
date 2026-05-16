use crate::analyze::resolve::relative_string;
use crate::analyze::routes::collect_layout_chain_files;
use crate::cli::Cli;
use crate::pipeline::aggregate::build_final_report;
use crate::pipeline::cache::Cache;
use crate::pipeline::route_analysis::{check_route_matches, collect_route_fetches};
use crate::pipeline::target::{resolve_target_file, TargetSpec};
use crate::report::types::{FinalReport, RouteReport};
use anyhow::Result;
use no_mistakes_core::{config, routes};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub(crate) fn run_with_base_root(base_root: &Path, cli: &Cli) -> Result<FinalReport> {
    let root = base_root.join(&cli.root);
    if !root.is_dir() {
        anyhow::bail!("root directory does not exist: {}", root.display());
    }
    let stems = [".no-mistakes", ".next-to-fetch"];
    let root_config: crate::report::types::RootConfig =
        config::load_config(&root, cli.config.as_deref(), &stems)?;
    let file_config = root_config.next_to_fetch.unwrap_or(root_config.legacy);

    let frontend_root_name = file_config
        .frontend_root
        .unwrap_or_else(|| "app".to_string());
    let frontend_root = root.join(&frontend_root_name);
    if !frontend_root.is_dir() {
        anyhow::bail!(
            "frontend root directory does not exist: {}",
            frontend_root.display()
        );
    }
    let stems = ["page", "route"];
    let all_routes = routes::collect_routes(&frontend_root, &stems);

    let mut cache = Cache {
        files: HashMap::new(),
        imports: HashMap::new(),
    };

    let target_specs = resolve_targets(base_root, &root, &cli.targets)?;
    let (reports, matched_targets) =
        analyze_routes(all_routes, &target_specs, &frontend_root, &root, &mut cache)?;

    verify_targets_matched(&target_specs, &matched_targets)?;

    Ok(build_final_report(reports))
}

fn resolve_targets(base_root: &Path, root: &Path, targets: &[String]) -> Result<Vec<TargetSpec>> {
    let mut target_specs = Vec::new();
    let mut unique_targets = HashSet::new();
    for target in targets {
        if unique_targets.insert(target.clone()) {
            // Targets that look like route patterns (e.g. "/users") won't resolve as files;
            // that's expected — `file: None` causes route-pattern matching downstream.
            let file = resolve_target_file(root, target)
                .or_else(|_| resolve_target_file(base_root, target))
                .ok();
            target_specs.push(TargetSpec {
                raw: target.clone(),
                file,
            });
        }
    }
    Ok(target_specs)
}

fn analyze_routes(
    all_routes: Vec<routes::Route>,
    target_specs: &[TargetSpec],
    frontend_root: &Path,
    root: &Path,
    cache: &mut Cache,
) -> Result<(Vec<RouteReport>, HashSet<String>)> {
    let mut reports = Vec::new();
    let mut matched_targets: HashSet<String> = HashSet::new();

    for route in all_routes {
        let route_is_page = route.file.file_stem().and_then(|s| s.to_str()) == Some("page");
        let wrapper_files = if route_is_page {
            collect_layout_chain_files(&route.file, frontend_root)
                .into_iter()
                .filter_map(|path| path.canonicalize().ok())
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        let (matched, newly_matched) =
            check_route_matches(&route, target_specs, &wrapper_files, cache)?;

        for t in newly_matched {
            matched_targets.insert(t);
        }

        if !matched {
            continue;
        }

        let fetches = collect_route_fetches(&route, frontend_root, root, cache)?;

        reports.push(RouteReport {
            route: route.pattern,
            file: relative_string(root, &route.file),
            api_calls: fetches,
        });
    }

    Ok((reports, matched_targets))
}

fn verify_targets_matched(
    target_specs: &[TargetSpec],
    matched_targets: &HashSet<String>,
) -> Result<()> {
    let unique_target_raws: HashSet<_> = target_specs.iter().map(|t| t.raw.as_str()).collect();
    let mut unmatched: Vec<_> = unique_target_raws
        .iter()
        .copied()
        .filter(|target| !matched_targets.contains(*target))
        .collect();
    if !unmatched.is_empty() {
        unmatched.sort();
        return Err(anyhow::anyhow!("Error: targets not found: {:?}", unmatched));
    }
    Ok(())
}
