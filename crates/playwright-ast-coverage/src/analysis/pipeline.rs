use crate::analysis::app_collect::collect_app_selector_occurrences;
use crate::analysis::context::TestAnalysisContext;
use crate::analysis::coverage::build_coverage;
use crate::analysis::discover::discover_test_files;
use crate::analysis::fetch::{collect_fetches_for_routes, expand_fetch_edges};
use crate::analysis::output::{
    build_related_report, print_coverage_text, print_edges_text, print_related_text,
};
use crate::analysis::routes_index::route_index;
use crate::analysis::selectors_index::{app_selector_targets, selector_index};
use crate::analysis::test_file::analyze_test_file;
use crate::analysis::tests_report::{build_tests_report, print_tests_text};
use crate::analysis::types::{Analysis, EdgeReport, UniqueSelectorPolicy};
use crate::cli::{Cli, Command};
use crate::config;
use crate::config::has_configured_html_id_selector;
use crate::fsutil::absolutize;
use crate::playwright_tests;
use crate::routes;
use crate::selectors;
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::path::Path;
use std::process::ExitCode;

pub fn run(cli: Cli) -> Result<ExitCode> {
    let root = absolutize(&cli.root).context("failed to resolve --root")?;
    let settings = config::load_settings(
        &root,
        cli.config.as_deref(),
        &cli.playwright_config,
        cli.project.clone(),
    )?;
    let analysis = analyze_with_policy(
        &root,
        &settings,
        playwright_tests::TestPolicy {
            assert_conditional_tests: cli.assert_conditional_tests,
            allow_skipped_tests: cli.allow_skipped_tests,
        },
        UniqueSelectorPolicy {
            test_ids: cli.assert_unique_test_ids || cli.assert_unique_selectors,
            html_ids: cli.assert_unique_html_ids
                || (cli.assert_unique_selectors && settings.html_ids),
            aggregate: cli.assert_unique_selectors,
            configured_html_id_selector: false,
        },
    )?;
    match cli.command {
        Command::Check => {
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&analysis.coverage)?);
            } else {
                print_coverage_text(&analysis.coverage);
            }
            if analysis.coverage.summary.uncovered_routes > 0
                || analysis.coverage.summary.uncovered_selectors > 0
                || analysis.coverage.summary.duplicate_selectors > 0
            {
                Ok(ExitCode::from(1))
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        Command::Edges => {
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&analysis.edges)?);
            } else {
                print_edges_text(&analysis.edges);
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Related { files } => {
            let related = build_related_report(&root, &analysis.edges.edges, &files);
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&related)?);
            } else {
                print_related_text(&related);
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Tests { files } => {
            let report = build_tests_report(&analysis.edges.edges, &files, &root);
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print_tests_text(&report);
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}

#[cfg(test)]
pub(crate) fn analyze(root: &Path, settings: &config::Settings) -> Result<Analysis> {
    analyze_with_policy(
        root,
        settings,
        playwright_tests::TestPolicy::default(),
        UniqueSelectorPolicy::default(),
    )
}

pub(crate) fn analyze_with_policy(
    root: &Path,
    settings: &config::Settings,
    test_policy: playwright_tests::TestPolicy,
    mut unique_selector_policy: UniqueSelectorPolicy,
) -> Result<Analysis> {
    unique_selector_policy.configured_html_id_selector = has_configured_html_id_selector(settings);
    let route_root = root.join(&settings.frontend_root);
    let routes = routes::collect_routes(&route_root)?;
    if routes.is_empty() {
        anyhow::bail!(
            "no Next.js page routes found under {}",
            route_root
                .strip_prefix(root)
                .unwrap_or(&route_root)
                .display()
        );
    }

    let playwright = crate::playwright_config::load_many(
        root,
        &settings.playwright_configs,
        settings.project.as_deref(),
    )?;
    let test_files = discover_test_files(root, settings, &playwright)?;
    let selector_regexes = selectors::compile_selector_regexes_with_html_ids(
        &settings.selector_attributes,
        &settings.component_selector_attributes,
        settings.html_ids,
    );
    let unique_html_id_scan = unique_selector_policy.html_ids && !settings.html_ids;
    let app_selector_regexes = selectors::compile_selector_regexes_with_html_ids(
        &settings.selector_attributes,
        &settings.component_selector_attributes,
        settings.html_ids || unique_html_id_scan,
    );
    let app_selector_occurrences = if settings.selector_attributes.is_empty()
        && settings.component_selector_attributes.is_empty()
        && !settings.html_ids
        && !unique_html_id_scan
    {
        Vec::new()
    } else {
        collect_app_selector_occurrences(root, settings, &app_selector_regexes)?
    };
    let mut app_selectors: Vec<_> = app_selector_occurrences
        .iter()
        .filter(|selector| {
            settings.html_ids
                || unique_selector_policy.configured_html_id_selector
                || selector.attribute != selectors::HTML_ID_ATTRIBUTE
        })
        .cloned()
        .collect();
    app_selectors.sort();
    app_selectors.dedup();
    let route_idx = route_index(root, &routes);
    let app_selector_tgts = app_selector_targets(root, &app_selectors);
    let selector_idx = selector_index(&app_selector_tgts);
    let test_analysis = TestAnalysisContext {
        root,
        route_index: &route_idx,
        app_selector_targets: &app_selector_tgts,
        selector_index: &selector_idx,
        navigation_helpers: &settings.navigation_helpers,
        selector_regexes: &selector_regexes,
        test_policy,
    };

    let mut edges = test_files
        .par_iter()
        .try_fold(Vec::new, |mut edges, test_file| -> Result<_> {
            edges.extend(analyze_test_file(test_file, &test_analysis)?);
            Ok(edges)
        })
        .try_reduce(Vec::new, |mut left, mut right| -> Result<_> {
            left.append(&mut right);
            Ok(left)
        })?;

    let fetch_idx = collect_fetches_for_routes(&routes, &route_root, root)?;
    edges.extend(expand_fetch_edges(&edges, &fetch_idx));
    edges.sort();
    edges.dedup();

    let edge_report = EdgeReport { edges };
    let coverage = build_coverage(
        root,
        &routes,
        &app_selectors,
        &app_selector_occurrences,
        &edge_report.edges,
        settings,
        unique_selector_policy,
        &fetch_idx,
    );
    Ok(Analysis {
        coverage,
        edges: edge_report,
    })
}
