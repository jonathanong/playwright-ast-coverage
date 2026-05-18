use super::{ExportBucket, ExportOrigin, SourceFile};
use crate::codebase::ts_resolver::{normalize_path, ImportResolver};
use crate::codebase::ts_symbols::{Export, ExportKind};
use crate::codebase::workspaces::WorkspaceMap;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub(super) fn find_target_export_origin(
    target: &Path,
    imported: &str,
    files: &HashMap<PathBuf, SourceFile>,
    resolver: &ImportResolver<'_>,
    workspace: &WorkspaceMap,
    visiting: &mut HashSet<PathBuf>,
) -> Option<ExportOrigin> {
    OriginSearch {
        files,
        resolver,
        workspace,
        visiting,
    }
    .find(target, imported)
}

struct OriginSearch<'a, 'b> {
    files: &'a HashMap<PathBuf, SourceFile>,
    resolver: &'a ImportResolver<'b>,
    workspace: &'a WorkspaceMap,
    visiting: &'a mut HashSet<PathBuf>,
}

impl OriginSearch<'_, '_> {
    fn find(&mut self, target: &Path, imported: &str) -> Option<ExportOrigin> {
        let target = normalize_path(target);
        if !self.visiting.insert(target.clone()) {
            return None;
        }
        let Some(file) = self.files.get(&target) else {
            self.visiting.remove(&target);
            return None;
        };
        if file.disabled {
            self.visiting.remove(&target);
            return None;
        }

        let found = file
            .symbols
            .exports
            .iter()
            .filter(|export| !super::collector::should_skip_export(file, export))
            .find_map(|export| self.find_export(file, export, imported));
        self.visiting.remove(&target);
        found
    }

    fn find_export(
        &mut self,
        file: &SourceFile,
        export: &Export,
        imported: &str,
    ) -> Option<ExportOrigin> {
        match &export.kind {
            ExportKind::Default if imported == "default" => Some(origin_for_export(
                file,
                export,
                ExportBucket::from_export(export),
            )),
            ExportKind::ReExport {
                source,
                imported: reimported,
            } if export.name == imported => {
                self.explicit_reexport_origin(file, export, source, reimported)
            }
            ExportKind::ReExport {
                source,
                imported: reimported,
            } if export.name == "*" && reimported == "*" => {
                resolve_export_source(source, &file.path, self.resolver, self.workspace)
                    .and_then(|resolved| self.find(&resolved, imported))
            }
            _ if export.name == imported => Some(origin_for_export(
                file,
                export,
                ExportBucket::from_export(export),
            )),
            _ => None,
        }
    }

    fn explicit_reexport_origin(
        &mut self,
        file: &SourceFile,
        export: &Export,
        source: &str,
        reimported: &str,
    ) -> Option<ExportOrigin> {
        let resolved_origin =
            match resolve_export_source(source, &file.path, self.resolver, self.workspace) {
                Some(resolved) => self.find(&resolved, reimported),
                None => None,
            };
        if export.is_type_only {
            if let Some(origin) = resolved_origin {
                Some(ExportOrigin {
                    bucket: ExportBucket::Type,
                    ..origin
                })
            } else {
                Some(origin_for_export(file, export, ExportBucket::Type))
            }
        } else {
            if resolved_origin.is_some() {
                resolved_origin
            } else {
                Some(origin_for_export(file, export, ExportBucket::Value))
            }
        }
    }
}

pub(super) fn origin_for_export(
    file: &SourceFile,
    export: &Export,
    bucket: ExportBucket,
) -> ExportOrigin {
    ExportOrigin {
        file: file.rel.clone(),
        line: export.line,
        name: export.name.clone(),
        bucket,
    }
}

pub(super) fn resolve_export_source(
    source: &str,
    importing_file: &Path,
    resolver: &ImportResolver<'_>,
    workspace: &WorkspaceMap,
) -> Option<PathBuf> {
    if let Some(path) = resolver.resolve(source, importing_file) {
        return Some(normalize_path(&path));
    }
    if let Some(path) = workspace.resolve_specifier(source) {
        return Some(normalize_path(&path));
    }
    None
}
