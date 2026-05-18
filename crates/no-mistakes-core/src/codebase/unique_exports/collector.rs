use super::{ExportBucket, ExportOccurrence, ExportOrigin, SourceFile, RULE_ID};
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
    memo: &mut HashMap<PathBuf, Vec<ExportOccurrence>>,
) -> Vec<ExportOccurrence> {
    let path = normalize_path(path);
    if let Some(cached) = memo.get(&path) {
        return cached.clone();
    }
    if !visiting.insert(path.clone()) {
        return Vec::new();
    }
    let Some(file) = files.get(&path) else {
        visiting.remove(&path);
        let out = Vec::new();
        memo.insert(path, out.clone());
        return out;
    };
    if file.disabled {
        visiting.remove(&path);
        let out = Vec::new();
        memo.insert(path, out.clone());
        return out;
    }

    let mut out = Vec::new();
    for export in &file.symbols.exports {
        if should_skip_export(file, export) {
            continue;
        }
        match &export.kind {
            ExportKind::Default => {}
            ExportKind::ReExport { source, imported } if export.name == "*" && imported == "*" => {
                let Some(target) = resolve_export_source(source, &file.path, resolver, workspace)
                else {
                    continue;
                };
                for mut occurrence in
                    collect_file_exports(&target, files, resolver, workspace, visiting, memo)
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
                let resolved = resolve_export_source(source, &file.path, resolver, workspace);
                let resolved_origin = resolved.as_ref().and_then(|target| {
                    find_target_export_origin(
                        target, imported, files, resolver, workspace, visiting,
                    )
                });
                let bucket = if export.is_type_only {
                    ExportBucket::Type
                } else if imported == "*" {
                    ExportBucket::Value
                } else {
                    resolved_origin
                        .as_ref()
                        .map(|origin| origin.bucket)
                        .unwrap_or_else(|| ExportBucket::from_export(export))
                };
                let origin = resolved_origin
                    .map(|origin| {
                        if export.is_type_only {
                            ExportOrigin {
                                bucket: ExportBucket::Type,
                                ..origin
                            }
                        } else {
                            origin
                        }
                    })
                    .unwrap_or_else(|| origin_for_export(file, export, bucket));
                out.push(ExportOccurrence {
                    name: export.name.clone(),
                    bucket,
                    file: file.rel.clone(),
                    line: export.line,
                    kind: export_kind_str(&export.kind).to_string(),
                    origin,
                });
            }
            _ => {
                let bucket = ExportBucket::from_export(export);
                out.push(ExportOccurrence {
                    name: export.name.clone(),
                    bucket,
                    file: file.rel.clone(),
                    line: export.line,
                    kind: export_kind_str(&export.kind).to_string(),
                    origin: origin_for_export(file, export, bucket),
                });
            }
        }
    }

    visiting.remove(&path);
    memo.insert(path, out.clone());
    out
}

fn should_skip_export(file: &SourceFile, export: &Export) -> bool {
    export.name == "default"
        || has_disable_comment(&file.source, export.line, RULE_ID)
        || super::nextjs::is_framework_export(&file.rel, &export.name, file.is_nextjs_project)
}

pub(super) fn find_target_export_origin(
    target: &Path,
    imported: &str,
    files: &HashMap<PathBuf, SourceFile>,
    resolver: &ImportResolver<'_>,
    workspace: &WorkspaceMap,
    visiting: &mut HashSet<PathBuf>,
) -> Option<ExportOrigin> {
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
            ExportKind::Default if imported == "default" => Some(origin_for_export(
                file,
                export,
                ExportBucket::from_export(export),
            )),
            ExportKind::ReExport {
                source,
                imported: reimported,
            } if export.name == imported => {
                let resolved_origin = resolve_export_source(
                    source, &file.path, resolver, workspace,
                )
                .and_then(|resolved| {
                    find_target_export_origin(
                        &resolved, reimported, files, resolver, workspace, visiting,
                    )
                });
                if export.is_type_only {
                    resolved_origin
                        .map(|origin| ExportOrigin {
                            bucket: ExportBucket::Type,
                            ..origin
                        })
                        .or_else(|| Some(origin_for_export(file, export, ExportBucket::Type)))
                } else {
                    resolved_origin
                        .or_else(|| Some(origin_for_export(file, export, ExportBucket::Value)))
                }
            }
            ExportKind::ReExport {
                source,
                imported: reimported,
            } if export.name == "*" && reimported == "*" => resolve_export_source(
                source, &file.path, resolver, workspace,
            )
            .and_then(|resolved| {
                find_target_export_origin(&resolved, imported, files, resolver, workspace, visiting)
            }),
            _ if export.name == imported => Some(origin_for_export(
                file,
                export,
                ExportBucket::from_export(export),
            )),
            _ => None,
        });
    visiting.remove(&target);
    found
}

fn origin_for_export(file: &SourceFile, export: &Export, bucket: ExportBucket) -> ExportOrigin {
    ExportOrigin {
        file: file.rel.clone(),
        line: export.line,
        name: export.name.clone(),
        bucket,
    }
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
