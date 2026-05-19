use crate::check_tasks::{
    run_codebase_check, run_filesystem_rules_check, run_integration_check, run_queue_check,
    run_react_check, run_rules_check, CheckTask,
};
use no_mistakes_core::codebase::check_facts::CheckFactMap;
use no_mistakes_core::codebase::rules::RuleFinding;
use no_mistakes_core::codebase::unique_exports::UniqueExportFinding;
use no_mistakes_core::integration_tests::IntegrationFinding;
use no_mistakes_core::queue::CheckFinding;
use no_mistakes_core::react_traits;
use std::path::{Path, PathBuf};

pub(crate) type DomainResults = (
    anyhow::Result<CheckTask<Vec<react_traits::Violation>>>,
    anyhow::Result<CheckTask<Vec<CheckFinding>>>,
    anyhow::Result<CheckTask<Vec<RuleFinding>>>,
    anyhow::Result<CheckTask<Vec<IntegrationFinding>>>,
    anyhow::Result<CheckTask<Vec<UniqueExportFinding>>>,
    anyhow::Result<CheckTask<Vec<RuleFinding>>>,
);

pub(crate) fn run_domain_checks(
    root: &Path,
    config_path: &Option<PathBuf>,
    tsconfig_path: &Option<PathBuf>,
    react_enabled: bool,
    queues_enabled: bool,
    unique_exports_enabled: bool,
    filesystem_rules_enabled: bool,
    discovered_files: Vec<PathBuf>,
    facts: &CheckFactMap,
) -> DomainResults {
    let ((react, queues), (rules, (integration, (codebase, filesystem_rules)))) = rayon::join(
        || {
            rayon::join(
                {
                    let root = root.to_path_buf();
                    let config = config_path.clone();
                    move || run_react_check(root, config, react_enabled, facts)
                },
                {
                    let root = root.to_path_buf();
                    let tsconfig = tsconfig_path.clone();
                    move || run_queue_check(root, tsconfig, queues_enabled, facts)
                },
            )
        },
        || {
            rayon::join(
                {
                    let root = root.to_path_buf();
                    let config = config_path.clone();
                    let tsconfig = tsconfig_path.clone();
                    move || run_rules_check(root, config, tsconfig, facts)
                },
                || {
                    rayon::join(
                        {
                            let root = root.to_path_buf();
                            let config = config_path.clone();
                            move || run_integration_check(root, config, facts)
                        },
                        || {
                            rayon::join(
                                {
                                    let root = root.to_path_buf();
                                    let config = config_path.clone();
                                    let tsconfig = tsconfig_path.clone();
                                    move || {
                                        run_codebase_check(
                                            root,
                                            config,
                                            tsconfig,
                                            unique_exports_enabled,
                                            facts,
                                        )
                                    }
                                },
                                {
                                    let root = root.to_path_buf();
                                    let config = config_path.clone();
                                    move || {
                                        run_filesystem_rules_check(
                                            root,
                                            config,
                                            filesystem_rules_enabled,
                                            discovered_files,
                                        )
                                    }
                                },
                            )
                        },
                    )
                },
            )
        },
    );
    (react, queues, rules, integration, codebase, filesystem_rules)
}
