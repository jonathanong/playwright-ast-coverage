use crate::codebase::config::load_codebase_config_with_path;
use crate::codebase::ts_resolver::{find_tsconfig, load_tsconfig, normalize_path, ImportResolver};
use crate::codebase::ts_source::discover_files;
use crate::codebase::workspaces;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

mod collector;
mod findings;
mod nextjs;
mod origin;
mod scan;
mod types;
mod with_facts;

use collector::collect_file_exports;
use findings::unique_export_findings;
use scan::{filter_source_files, sorted_paths};
use types::{ExportBucket, ExportOccurrence, ExportOrigin, SourceFile};
pub use types::{UniqueExportFinding, UniqueExportsOptions};
pub use with_facts::analyze_project_with_facts;

pub const RULE_ID: &str = "unique-exports";

pub fn analyze_project(
    root: &Path,
    config_path: Option<&Path>,
    tsconfig_path: Option<&Path>,
) -> Result<Vec<UniqueExportFinding>> {
    let root = normalize_path(root);
    let root = root.as_path();
    let config = load_codebase_config_with_path(root, config_path)?;
    let mut workspace_files = discover_files(root, &config.filesystem.skip_directories);
    for project_root in config.project_roots_for_rule(root, RULE_ID) {
        workspace_files.extend(discover_files(
            &project_root,
            &config.filesystem.skip_directories,
        ));
    }
    workspace_files.sort();
    workspace_files.dedup();
    let facts = crate::codebase::check_facts::collect_check_facts(
        root,
        workspace_files,
        crate::codebase::check_facts::CheckFactPlan {
            source: true,
            symbols: true,
            ..Default::default()
        },
    );
    with_facts::analyze_project_with_facts(root, config_path, tsconfig_path, &facts)
}

fn analyze_unique_exports(
    _root: &Path,
    analysis_files: Vec<PathBuf>,
    source_files: Vec<SourceFile>,
    options: UniqueExportsOptions,
    resolver: ImportResolver<'_>,
    workspace: crate::codebase::workspaces::WorkspaceMap,
) -> Result<Vec<UniqueExportFinding>> {
    let by_path: HashMap<PathBuf, SourceFile> = source_files
        .into_iter()
        .map(|file| (file.path.clone(), file))
        .collect();

    let mut occurrences = Vec::new();
    let mut export_memo = HashMap::new();
    for path in sorted_paths(analysis_files.iter()) {
        let mut visiting = HashSet::new();
        occurrences.extend(collect_file_exports(
            path,
            &by_path,
            &resolver,
            &workspace,
            &mut visiting,
            &mut export_memo,
        ));
    }

    unique_export_findings(occurrences, options)
}

trait DefaultTsConfig {
    fn unwrap_or_default_for(self, root: &Path) -> crate::codebase::ts_resolver::TsConfig;
}

impl DefaultTsConfig for Option<crate::codebase::ts_resolver::TsConfig> {
    fn unwrap_or_default_for(self, root: &Path) -> crate::codebase::ts_resolver::TsConfig {
        self.unwrap_or_else(|| crate::codebase::ts_resolver::TsConfig {
            dir: root.to_path_buf(),
            paths: Vec::new(),
            paths_dir: root.to_path_buf(),
            base_url: None,
        })
    }
}

#[cfg(test)]
mod tests;
