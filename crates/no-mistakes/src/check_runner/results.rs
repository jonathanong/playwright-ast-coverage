use crate::check_parallel::DomainResults;
use crate::check_tasks::CheckTask;
use anyhow::Result;
use no_mistakes_core::codebase::rules::RuleFinding;
use no_mistakes_core::codebase::unique_exports::UniqueExportFinding;
use no_mistakes_core::integration_tests::IntegrationFinding;
use no_mistakes_core::queue::CheckFinding;
use no_mistakes_core::react_traits;
use std::time::Duration;

pub(crate) struct CheckResults {
    pub(crate) react: Vec<react_traits::Violation>,
    pub(crate) queues: Vec<CheckFinding>,
    pub(crate) rules: Vec<RuleFinding>,
    pub(crate) integration: Vec<IntegrationFinding>,
    pub(crate) codebase: Vec<UniqueExportFinding>,
    pub(crate) warnings: Vec<String>,
    pub(crate) timings: Vec<(&'static str, Duration)>,
}

pub(crate) struct CompletedDomainChecks {
    pub(crate) react: CheckTask<Vec<react_traits::Violation>>,
    pub(crate) queues: CheckTask<Vec<CheckFinding>>,
    pub(crate) rules: CheckTask<Vec<RuleFinding>>,
    pub(crate) integration: CheckTask<Vec<IntegrationFinding>>,
    pub(crate) codebase: CheckTask<Vec<UniqueExportFinding>>,
    pub(crate) filesystem_rules: CheckTask<Vec<RuleFinding>>,
}

impl CheckResults {
    pub(crate) fn has_findings(&self) -> bool {
        !self.react.is_empty()
            || !self.queues.is_empty()
            || !self.rules.is_empty()
            || !self.integration.is_empty()
            || !self.codebase.is_empty()
    }
}

pub(crate) fn complete_domain_checks(results: DomainResults) -> Result<CompletedDomainChecks> {
    let (react, queues, rules, integration, codebase, filesystem_rules) = results;
    Ok(CompletedDomainChecks {
        react: react?,
        queues: queues?,
        rules: rules?,
        integration: integration?,
        codebase: codebase?,
        filesystem_rules: filesystem_rules?,
    })
}

pub(crate) fn empty_results(warnings: [Option<String>; 1]) -> CheckResults {
    let warnings = warnings.into_iter().flatten().collect();
    CheckResults {
        react: Vec::new(),
        queues: Vec::new(),
        rules: Vec::new(),
        integration: Vec::new(),
        codebase: Vec::new(),
        warnings,
        timings: vec![
            ("discover", Duration::ZERO),
            ("parse_extract", Duration::ZERO),
            ("react", Duration::ZERO),
            ("queues", Duration::ZERO),
            ("rules", Duration::ZERO),
            ("integration", Duration::ZERO),
            ("codebase", Duration::ZERO),
            ("filesystem_rules", Duration::ZERO),
        ],
    }
}
