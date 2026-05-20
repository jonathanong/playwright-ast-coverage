use crate::check_parallel::run_domain_checks;
use crate::check_tasks::{
    filesystem_rules_configured, queues_configured, unique_exports_configured,
};
use anyhow::Result;
use no_mistakes_core::codebase::check_facts::{collect_check_facts, CheckFactPlan};
use no_mistakes_core::config::v2::load_v2_config;
use no_mistakes_core::react_traits;
use std::path::PathBuf;
use std::time::Instant;

mod results;

pub(crate) use results::CheckResults;
use results::{complete_domain_checks, empty_results};

pub(crate) fn run_all(
    root: PathBuf,
    config_path: Option<PathBuf>,
    tsconfig_path: Option<PathBuf>,
) -> Result<CheckResults> {
    let root = root.canonicalize().unwrap_or(root);
    let config = load_v2_config(&root, config_path.as_deref())?;
    let queues_enabled = queues_configured(&config);
    let unique_exports_enabled = unique_exports_configured(&config);
    let rules_enabled = test_dynamic_imports_configured(&config);
    let filesystem_rules_enabled = filesystem_rules_configured(&config);
    let integration_enabled = integration_configured(&config);
    let react_enabled = react_traits::check_enabled(&root, config_path.as_deref(), false)?;
    let react_warning = None;
    let plan = fact_plan(
        react_enabled,
        queues_enabled,
        rules_enabled,
        integration_enabled,
        unique_exports_enabled,
    );
    if !plan_requests_facts(&plan) && !filesystem_rules_enabled {
        return Ok(empty_results([react_warning]));
    }
    let discover_start = Instant::now();
    let skip_directories = config.filesystem.skip_directories.clone();
    let discovered = crate::check_discovery::discover_check_files(
        &root,
        &config,
        &skip_directories,
        unique_exports_enabled,
    );
    let discover_duration = discover_start.elapsed();
    let facts_start = Instant::now();
    // When only filesystem rules are enabled, no TS/JS parsing is needed.
    let (fs_files, facts) = if plan_requests_facts(&plan) {
        let fs = if filesystem_rules_enabled {
            discovered.clone()
        } else {
            Vec::new()
        };
        let f = collect_check_facts(&root, discovered, plan);
        (fs, f)
    } else {
        (discovered, Default::default())
    };
    let facts_duration = facts_start.elapsed();

    let (react, queues, rules, integration, codebase, filesystem_rules) = run_domain_checks(
        &root,
        &config_path,
        &tsconfig_path,
        react_enabled,
        queues_enabled,
        unique_exports_enabled,
        filesystem_rules_enabled,
        fs_files,
        &facts,
    );

    let completed = complete_domain_checks((
        react,
        queues,
        rules,
        integration,
        codebase,
        filesystem_rules,
    ))?;
    let react = completed.react;
    let queues = completed.queues;
    let mut rules = completed.rules;
    let integration = completed.integration;
    let codebase = completed.codebase;
    let filesystem_rules = completed.filesystem_rules;
    let warnings = [
        react_warning,
        react.warning.clone(),
        queues.warning.clone(),
        rules.warning.clone(),
        integration.warning.clone(),
        codebase.warning.clone(),
        filesystem_rules.warning.clone(),
    ]
    .into_iter()
    .flatten()
    .collect();

    rules.findings.extend(filesystem_rules.findings);

    Ok(CheckResults {
        timings: vec![
            ("discover", discover_duration),
            ("parse_extract", facts_duration),
            ("react", react.duration),
            ("queues", queues.duration),
            ("rules", rules.duration),
            ("integration", integration.duration),
            ("codebase", codebase.duration),
            ("filesystem_rules", filesystem_rules.duration),
        ],
        react: react.findings,
        queues: queues.findings,
        rules: rules.findings,
        integration: integration.findings,
        codebase: codebase.findings,
        warnings,
    })
}

fn fact_plan(
    react: bool,
    queue: bool,
    rules: bool,
    integration: bool,
    unique_exports: bool,
) -> CheckFactPlan {
    CheckFactPlan {
        imports: rules,
        symbols: unique_exports,
        react,
        queue,
        integration,
        dynamic_imports: rules,
        source: rules || unique_exports,
    }
}

fn plan_requests_facts(plan: &CheckFactPlan) -> bool {
    plan.imports
        || plan.symbols
        || plan.react
        || plan.queue
        || plan.integration
        || plan.dynamic_imports
        || plan.source
}

fn test_dynamic_imports_configured(
    config: &no_mistakes_core::config::v2::NoMistakesConfig,
) -> bool {
    crate::check_tasks::rule_configured(
        config,
        no_mistakes_core::codebase::rules::TEST_NO_UNMOCKED_DYNAMIC_IMPORTS,
    )
}

fn integration_configured(config: &no_mistakes_core::config::v2::NoMistakesConfig) -> bool {
    let vitest_configured = !config.tests.vitest.suites.is_empty();
    let playwright_configured = !config.tests.playwright.suites.is_empty();
    if vitest_configured {
        return true;
    }
    if playwright_configured {
        return true;
    }
    false
}

#[cfg(test)]
mod tests;
