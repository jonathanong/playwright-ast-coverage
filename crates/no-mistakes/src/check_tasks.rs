use anyhow::Result;
use no_mistakes_core::codebase::check_facts::CheckFactMap;
use no_mistakes_core::codebase::rules::{self, RuleFinding};
use no_mistakes_core::codebase::unique_exports::{self, UniqueExportFinding};
use no_mistakes_core::config::v2::NoMistakesConfig;
use no_mistakes_core::integration_tests::{self, IntegrationFinding};
use no_mistakes_core::queue::CheckFinding;
use no_mistakes_core::react_traits;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub(crate) struct CheckTask<T> {
    pub(crate) findings: T,
    pub(crate) warning: Option<String>,
    pub(crate) duration: Duration,
}

pub(crate) fn run_react_check(
    root: PathBuf,
    config: Option<PathBuf>,
    enabled: bool,
    facts: &CheckFactMap,
) -> Result<CheckTask<Vec<react_traits::Violation>>> {
    let start = Instant::now();
    let (findings, warning) = if enabled {
        match react_traits::run_check_with_facts(&root, config.as_deref(), &[], false, facts) {
            Ok(findings) => (findings, None),
            Err(err) => (
                Vec::new(),
                Some(format!("warning: react check skipped: {err:#}")),
            ),
        }
    } else {
        (Vec::new(), None)
    };
    Ok(CheckTask {
        findings,
        warning,
        duration: start.elapsed(),
    })
}

pub(crate) fn run_queue_check(
    root: PathBuf,
    tsconfig: Option<PathBuf>,
    enabled: bool,
    facts: &CheckFactMap,
) -> Result<CheckTask<Vec<CheckFinding>>> {
    let start = Instant::now();
    let findings = if enabled {
        no_mistakes_core::queue::analyze_project_with_facts(&root, tsconfig.as_deref(), &[], facts)?
            .check
    } else {
        Vec::new()
    };
    Ok(CheckTask {
        findings,
        warning: None,
        duration: start.elapsed(),
    })
}

pub(crate) fn run_rules_check(
    root: PathBuf,
    config: Option<PathBuf>,
    tsconfig: Option<PathBuf>,
    facts: &CheckFactMap,
) -> Result<CheckTask<Vec<RuleFinding>>> {
    let start = Instant::now();
    let (findings, warning) =
        match rules::run_check_with_facts(&root, config.as_deref(), tsconfig.as_deref(), facts) {
            Ok(findings) => (findings, None),
            Err(err) => (
                Vec::new(),
                Some(format!("warning: rules check skipped: {err:#}")),
            ),
        };
    Ok(CheckTask {
        findings,
        warning,
        duration: start.elapsed(),
    })
}

pub(crate) fn run_integration_check(
    root: PathBuf,
    config: Option<PathBuf>,
    facts: &CheckFactMap,
) -> Result<CheckTask<Vec<IntegrationFinding>>> {
    let start = Instant::now();
    let findings = integration_tests::check_with_facts(&root, config.as_deref(), facts)?;
    Ok(CheckTask {
        findings,
        warning: None,
        duration: start.elapsed(),
    })
}

pub(crate) fn run_codebase_check(
    root: PathBuf,
    config: Option<PathBuf>,
    tsconfig: Option<PathBuf>,
    enabled: bool,
    facts: &CheckFactMap,
) -> Result<CheckTask<Vec<UniqueExportFinding>>> {
    let start = Instant::now();
    let findings = if enabled {
        unique_exports::analyze_project_with_facts(
            &root,
            config.as_deref(),
            tsconfig.as_deref(),
            facts,
        )?
    } else {
        Vec::new()
    };
    Ok(CheckTask {
        findings,
        warning: None,
        duration: start.elapsed(),
    })
}

pub(crate) fn queues_configured(config: &NoMistakesConfig) -> bool {
    config
        .projects
        .values()
        .any(|project| !project.queues.enqueues.is_empty() || !project.queues.workers.is_empty())
}

pub(crate) fn unique_exports_configured(config: &NoMistakesConfig) -> bool {
    rule_configured(config, unique_exports::RULE_ID)
}

pub(crate) fn rule_configured(config: &NoMistakesConfig, rule_id: &str) -> bool {
    if config.rules.get(rule_id).is_some_and(|rule| !rule.enabled) {
        return false;
    }
    config.rules.contains_key(rule_id)
        || config
            .projects
            .values()
            .any(|project| project.rules.iter().any(|rule| rule == rule_id))
}
