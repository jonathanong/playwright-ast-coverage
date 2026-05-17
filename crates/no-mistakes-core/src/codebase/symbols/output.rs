use super::{export_kind_str, FileEntry, ResolvedExport, ResolvedImport};
use crate::codebase::ts_symbols::ExportKind;
use anyhow::Result;
use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;

/// For human-friendly output formats (md, human), prefer the resolved
/// project-relative path so an agent can chase the import without re-resolving
/// the specifier. Falls back to the raw specifier when resolution failed (bare
/// npm packages, etc.).
fn display_source(resolved: &Option<PathBuf>, fallback: &str) -> String {
    resolved
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| fallback.to_string())
}

// ── Shared serializable shape ────────────────────────────────────────────
//
// One owned struct family powers both JSON and YAML. Owning the strings
// (rather than borrowing) costs an extra allocation per field at emit time
// but cuts the duplication that previously embedded near-identical structs
// inside `write_yml`.

#[derive(Serialize)]
struct ReExportInfo {
    source: String,
    imported: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolved: Option<String>,
}

#[derive(Serialize)]
struct ExportEntry {
    name: String,
    kind: &'static str,
    line: u32,
    #[serde(skip_serializing_if = "Option::is_none", rename = "reExport")]
    re_export: Option<ReExportInfo>,
}

#[derive(Serialize)]
struct ImportEntry {
    source: String,
    imported: String,
    local: String,
    line: u32,
    #[serde(rename = "typeOnly")]
    type_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolved: Option<String>,
}

#[derive(Serialize)]
struct FileOutput {
    path: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    exports: Vec<ExportEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    imports: Vec<ImportEntry>,
}

#[derive(Serialize)]
struct Output {
    roots: Vec<String>,
    files: Vec<FileOutput>,
}

fn export_to_entry(e: &ResolvedExport) -> ExportEntry {
    let re_export = match &e.kind {
        ExportKind::ReExport { source, imported } => Some(ReExportInfo {
            source: source.clone(),
            imported: imported.clone(),
            resolved: e.resolved.as_ref().map(|p| p.display().to_string()),
        }),
        _ => None,
    };
    ExportEntry {
        name: e.name.clone(),
        kind: export_kind_str(&e.kind),
        line: e.line,
        re_export,
    }
}

fn import_to_entry(i: &ResolvedImport) -> ImportEntry {
    ImportEntry {
        source: i.source.clone(),
        imported: i.imported.clone(),
        local: i.local.clone(),
        line: i.line,
        type_only: i.is_type_only,
        resolved: i.resolved.as_ref().map(|p| p.display().to_string()),
    }
}

fn build_output(roots: &[String], entries: &[FileEntry]) -> Output {
    Output {
        roots: roots.to_vec(),
        files: entries
            .iter()
            .map(|e| FileOutput {
                path: e.rel_path.display().to_string(),
                exports: e.exports.iter().map(export_to_entry).collect(),
                imports: e.imports.iter().map(import_to_entry).collect(),
            })
            .collect(),
    }
}

pub fn write_json(roots: &[String], entries: &[FileEntry], w: &mut dyn Write) -> Result<()> {
    let out = build_output(roots, entries);
    serde_json::to_writer_pretty(&mut *w, &out)?;
    writeln!(w)?;
    Ok(())
}

pub fn write_yml(roots: &[String], entries: &[FileEntry], w: &mut dyn Write) -> Result<()> {
    let out = build_output(roots, entries);
    serde_yaml::to_writer(w, &out)?;
    Ok(())
}

pub fn write_md(roots: &[String], entries: &[FileEntry], w: &mut dyn Write) -> Result<()> {
    if roots.len() == 1 {
        writeln!(w, "# `{}`", roots[0])?;
    } else {
        writeln!(w, "# {} files", roots.len())?;
        for root in roots {
            writeln!(w, "- `{root}`")?;
        }
    }
    writeln!(w)?;

    if entries
        .iter()
        .all(|e| e.exports.is_empty() && e.imports.is_empty())
    {
        writeln!(w, "_No symbols found._")?;
        return Ok(());
    }

    for entry in entries {
        if entries.len() > 1 {
            writeln!(w, "## `{}`", entry.rel_path.display())?;
        }
        for heading in ["### Exports"]
            .iter()
            .take(usize::from(!entry.exports.is_empty()))
        {
            writeln!(w, "{heading}")?;
        }
        for e in &entry.exports {
            let kind = export_kind_str(&e.kind);
            if let ExportKind::ReExport { source, imported } = &e.kind {
                let src = display_source(&e.resolved, source);
                let line = format!(
                    "- `{}` ({}, line {}) - re-exports `{}` from `{}`",
                    e.name, kind, e.line, imported, src
                );
                writeln!(w, "{line}")?;
            } else {
                writeln!(w, "- `{}` ({}, line {})", e.name, kind, e.line)?;
            }
        }
        if !entry.imports.is_empty() {
            writeln!(w, "### Imports")?;
            for i in &entry.imports {
                let type_tag = if i.is_type_only { " (type-only)" } else { "" };
                let src = display_source(&i.resolved, &i.source);
                if i.imported == i.local {
                    let line = format!(
                        "- `{}` from `{}` (line {}){}",
                        i.imported, src, i.line, type_tag
                    );
                    writeln!(w, "{line}")?;
                } else {
                    let line = format!(
                        "- `{}` as `{}` from `{}` (line {}){}",
                        i.imported, i.local, src, i.line, type_tag
                    );
                    writeln!(w, "{line}")?;
                }
            }
            writeln!(w)?;
        }
    }
    Ok(())
}

pub fn write_paths(entries: &[FileEntry], w: &mut dyn Write) -> Result<()> {
    for entry in entries {
        let path = entry.rel_path.display();
        for e in &entry.exports {
            writeln!(w, "{}:{}:{}", path, e.line, e.name)?;
        }
        for i in &entry.imports {
            writeln!(w, "{}:{}:{}", path, i.line, i.local)?;
        }
    }
    Ok(())
}

pub fn write_human(roots: &[String], entries: &[FileEntry], w: &mut dyn Write) -> Result<()> {
    if roots.len() == 1 {
        writeln!(w, "{}", roots[0])?;
    } else {
        writeln!(w, "{} files", roots.len())?;
    }

    if entries
        .iter()
        .all(|e| e.exports.is_empty() && e.imports.is_empty())
    {
        writeln!(w, "  (no symbols)")?;
        return Ok(());
    }

    for (idx, entry) in entries.iter().enumerate() {
        if entries.len() > 1 {
            if idx > 0 {
                writeln!(w)?;
            }
            writeln!(w, "{}", entry.rel_path.display())?;
        }
        for e in &entry.exports {
            let kind = export_kind_str(&e.kind);
            match &e.kind {
                ExportKind::ReExport { source, imported } => {
                    let src = display_source(&e.resolved, source);
                    let line = format!(
                        "  export {:<10} {:<24} :{:<4} <- {} from {}",
                        kind, e.name, e.line, imported, src
                    );
                    writeln!(w, "{line}")?;
                }
                _ => {
                    writeln!(w, "  export {:<10} {:<24} :{}", kind, e.name, e.line)?;
                }
            }
        }
        for i in &entry.imports {
            let type_tag = if i.is_type_only { " (type)" } else { "" };
            let lhs = if i.imported == i.local {
                i.imported.clone()
            } else {
                format!("{} as {}", i.imported, i.local)
            };
            let src = display_source(&i.resolved, &i.source);
            let line = format!(
                "  import{} {:<24} :{:<4} from {}",
                type_tag, lhs, i.line, src
            );
            writeln!(w, "{line}")?;
        }
    }
    Ok(())
}
