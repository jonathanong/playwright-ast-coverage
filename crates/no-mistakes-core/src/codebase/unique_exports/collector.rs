use super::{ExportBucket, ExportOccurrence, SourceFile, RULE_ID};
use crate::codebase::symbols::export_kind_str;
use crate::codebase::ts_resolver::{normalize_path, ImportResolver};
use crate::codebase::ts_source::has_disable_comment;
use crate::codebase::ts_symbols::{Export, ExportKind};
use crate::codebase::workspaces::WorkspaceMap;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub(super) fn collect_file_exports(
    path: &Path,
    files: &HashMap<PathBuf, SourceFile>,
    resolver: &ImportResolver<'_>,
    workspace: &WorkspaceMap,
    visiting: &mut HashSet<PathBuf>,
) -> Vec<ExportOccurrence> {
    let path = normalize_path(path);
    if !visiting.insert(path.clone()) {
        return Vec::new();
    }
    let Some(file) = files.get(&path) else {
        visiting.remove(&path);
        return Vec::new();
    };
    if file.disabled {
        visiting.remove(&path);
        return Vec::new();
    }

    let mut out = Vec::new();
    for export in &file.symbols.exports {
        if should_skip_export(file, export) {
            continue;
        }
        match &export.kind {
            ExportKind::Default => {}
            ExportKind::ReExport { source, imported } if imported == "*" => {
                let Some(target) = resolve_export_source(source, &file.path, resolver, workspace)
                else {
                    continue;
                };
                for mut occurrence in
                    collect_file_exports(&target, files, resolver, workspace, visiting)
                {
                    if export.is_type_only {
                        if occurrence.bucket == ExportBucket::Value {
                            continue;
                        }
                        occurrence.bucket = ExportBucket::Type;
                    }
                    occurrence.file = file.rel.clone();
                    occurrence.line = export.line;
                    occurrence.kind = export_kind_str(&export.kind).to_string();
                    if !super::nextjs::is_framework_export(
                        &occurrence.file,
                        &occurrence.name,
                        file.is_nextjs_project,
                    ) {
                        out.push(occurrence);
                    }
                }
            }
            ExportKind::ReExport { source, imported } => {
                let bucket = if export.is_type_only {
                    ExportBucket::Type
                } else {
                    resolve_export_source(source, &file.path, resolver, workspace)
                        .and_then(|target| {
                            find_target_export_bucket(
                                &target, imported, files, resolver, workspace, visiting,
                            )
                        })
                        .unwrap_or_else(|| ExportBucket::from_export(export))
                };
                out.push(ExportOccurrence {
                    name: export.name.clone(),
                    bucket,
                    file: file.rel.clone(),
                    line: export.line,
                    kind: export_kind_str(&export.kind).to_string(),
                });
            }
            _ => {
                out.push(ExportOccurrence {
                    name: export.name.clone(),
                    bucket: ExportBucket::from_export(export),
                    file: file.rel.clone(),
                    line: export.line,
                    kind: export_kind_str(&export.kind).to_string(),
                });
            }
        }
    }

    visiting.remove(&path);
    out
}

fn should_skip_export(file: &SourceFile, export: &Export) -> bool {
    has_disable_comment(&file.source, export.line, RULE_ID)
        || super::nextjs::is_framework_export(&file.rel, &export.name, file.is_nextjs_project)
}

pub(super) fn find_target_export_bucket(
    target: &Path,
    imported: &str,
    files: &HashMap<PathBuf, SourceFile>,
    resolver: &ImportResolver<'_>,
    workspace: &WorkspaceMap,
    visiting: &mut HashSet<PathBuf>,
) -> Option<ExportBucket> {
    let target = normalize_path(target);
    if !visiting.insert(target.clone()) {
        return None;
    }
    let Some(file) = files.get(&target) else {
        visiting.remove(&target);
        return None;
    };
    if file.disabled {
        visiting.remove(&target);
        return None;
    }

    let found = file
        .symbols
        .exports
        .iter()
        .filter(|export| !should_skip_export(file, export))
        .find_map(|export| match &export.kind {
            ExportKind::Default if imported == "default" => Some(ExportBucket::from_export(export)),
            ExportKind::ReExport {
                source,
                imported: reimported,
            } if export.name == imported => {
                if export.is_type_only {
                    Some(ExportBucket::Type)
                } else {
                    resolve_export_source(source, &file.path, resolver, workspace)
                        .and_then(|resolved| {
                            find_target_export_bucket(
                                &resolved, reimported, files, resolver, workspace, visiting,
                            )
                        })
                        .or(Some(ExportBucket::Value))
                }
            }
            ExportKind::ReExport {
                source,
                imported: reimported,
            } if reimported == "*" => resolve_export_source(
                source, &file.path, resolver, workspace,
            )
            .and_then(|resolved| {
                find_target_export_bucket(&resolved, imported, files, resolver, workspace, visiting)
            }),
            _ if export.name == imported => Some(ExportBucket::from_export(export)),
            _ => None,
        });
    visiting.remove(&target);
    found
}

fn resolve_export_source(
    source: &str,
    importing_file: &Path,
    resolver: &ImportResolver<'_>,
    workspace: &WorkspaceMap,
) -> Option<PathBuf> {
    resolver
        .resolve(source, importing_file)
        .or_else(|| workspace.resolve_specifier(source))
        .map(|path| normalize_path(&path))
}
