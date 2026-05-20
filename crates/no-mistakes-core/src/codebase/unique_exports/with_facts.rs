use super::{analyze_unique_exports, filter_source_files, load_codebase_config_with_path};
use super::{find_tsconfig, load_tsconfig, normalize_path, workspaces, DefaultTsConfig};
use super::{ImportResolver, UniqueExportFinding, UniqueExportsOptions, RULE_ID};
use crate::codebase::check_facts::CheckFactMap;
use anyhow::Result;
use std::path::Path;

pub fn analyze_project_with_facts(
    root: &Path,
    config_path: Option<&Path>,
    tsconfig_path: Option<&Path>,
    shared: &CheckFactMap,
) -> Result<Vec<UniqueExportFinding>> {
    let root = normalize_path(root);
    let root = root.as_path();
    let config = load_codebase_config_with_path(root, config_path)?;
    let project_roots = config
        .project_roots_for_rule(root, RULE_ID)
        .into_iter()
        .map(|path| normalize_path(&path))
        .collect::<Vec<_>>();
    if project_roots.is_empty() {
        return Ok(Vec::new());
    }
    let options: UniqueExportsOptions = config.rule_options(RULE_ID);
    let workspace_files = shared.files().to_vec();
    let mut analysis_files = workspace_files
        .iter()
        .filter(|path| {
            project_roots
                .iter()
                .any(|project_root| path.starts_with(project_root))
        })
        .cloned()
        .collect::<Vec<_>>();
    analysis_files.sort();
    analysis_files.dedup();
    let analysis_files = filter_source_files(&analysis_files);
    let symbol_files = shared_symbol_files(&workspace_files, &analysis_files, &config);
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
    let workspace = workspaces::load_from_files(root, &workspace_files).unwrap_or_default();
    let source_files = super::scan::collect_source_files_from_facts(root, &symbol_files, shared)?;
    analyze_unique_exports(
        root,
        analysis_files,
        source_files,
        options,
        resolver,
        workspace,
    )
}

fn shared_symbol_files(
    workspace_files: &[std::path::PathBuf],
    analysis_files: &[std::path::PathBuf],
    _config: &crate::codebase::config::Config,
) -> Vec<std::path::PathBuf> {
    let mut symbol_files = workspace_files.to_vec();
    symbol_files.extend(analysis_files.iter().cloned());
    symbol_files.sort();
    symbol_files.dedup();
    filter_source_files(&symbol_files)
}
