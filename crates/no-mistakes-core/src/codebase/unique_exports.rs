use crate::codebase::config::load_codebase_config_with_path;
use crate::codebase::ts_resolver::{find_tsconfig, load_tsconfig, normalize_path, ImportResolver};
use crate::codebase::ts_source::discover_files;
use crate::codebase::workspaces;
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

mod collector;
mod nextjs;
mod origin;
mod scan;
mod types;

use collector::collect_file_exports;
use scan::{collect_source_files, filter_source_files, sorted_paths};
use types::{ExportBucket, ExportOccurrence, ExportOrigin, SourceFile};
pub use types::{UniqueExportFinding, UniqueExportsOptions};

pub const RULE_ID: &str = "unique-exports";

pub fn analyze_project(
    root: &Path,
    config_path: Option<&Path>,
    tsconfig_path: Option<&Path>,
) -> Result<Vec<UniqueExportFinding>> {
    let root = normalize_path(root);
    let root = root.as_path();
    let config = load_codebase_config_with_path(root, config_path)?;
    let project_roots = config.project_roots_for_rule(root, RULE_ID);
    if project_roots.is_empty() {
        return Ok(Vec::new());
    }
    let options: UniqueExportsOptions = config.rule_options(RULE_ID);
    let mut all_files = Vec::new();
    for project_root in &project_roots {
        all_files.extend(discover_files(
            project_root,
            &config.filesystem.skip_directories,
        ));
    }
    all_files.sort();
    all_files.dedup();
    let files = filter_source_files(root, &all_files, &config.filesystem.skip_file_patterns)?;
    let tsconfig = match tsconfig_path {
        Some(path) => {
            let path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                root.join(path)
            };
            load_tsconfig(&path)?
        }
        None => find_tsconfig(root)
            .map(|path| load_tsconfig(&path))
            .transpose()?
            .unwrap_or_default_for(root),
    };
    let resolver = ImportResolver::new(&tsconfig);
    let workspace = workspaces::load_from_files(root, &all_files).unwrap_or_default();
    let source_files = collect_source_files(root, &files)?;
    let by_path: HashMap<PathBuf, SourceFile> = source_files
        .into_iter()
        .map(|file| (file.path.clone(), file))
        .collect();

    let mut occurrences = Vec::new();
    let mut export_memo = HashMap::new();
    for path in sorted_paths(by_path.keys()) {
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

    let mut buckets: BTreeMap<(String, ExportBucket), Vec<ExportOccurrence>> = BTreeMap::new();
    for occurrence in occurrences {
        buckets
            .entry((
                occurrence.name.clone(),
                occurrence
                    .bucket
                    .key(options.unique_across_types_and_values),
            ))
            .or_default()
            .push(occurrence);
    }

    let mut findings = Vec::new();
    for ((name, bucket), mut occurrences) in buckets {
        occurrences.sort_by(|a, b| (&a.file, a.line, &a.kind).cmp(&(&b.file, b.line, &b.kind)));
        let mut origins = BTreeSet::new();
        let unique_occurrences = occurrences
            .into_iter()
            .filter(|occurrence| origins.insert(occurrence.origin.clone()))
            .collect::<Vec<_>>();
        if unique_occurrences.len() < 2 {
            continue;
        }
        let first = &unique_occurrences[0];
        for duplicate in unique_occurrences.iter().skip(1) {
            findings.push(UniqueExportFinding {
                rule: RULE_ID.to_string(),
                file: duplicate.file.clone(),
                line: duplicate.line,
                export_name: name.clone(),
                export_kind: bucket.as_str().to_string(),
                message: format!(
                    "{} `{}` is already exported from {}:{}; rename or consolidate this exported API",
                    bucket.message_label(),
                    name,
                    first.file,
                    first.line
                ),
            });
        }
    }
    findings.sort();
    findings.dedup();
    Ok(findings)
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
