use super::graph::{EdgeKind, NodeEntry, NodeId};
use anyhow::Result;
use serde::Serialize;
use std::io::Write;
use std::path::Path;

fn edge_kind_str(k: EdgeKind) -> &'static str {
    match k {
        EdgeKind::Import => "import",
        EdgeKind::TypeImport => "type-import",
        EdgeKind::DynamicImport => "dynamic-import",
        EdgeKind::Require => "require",
        EdgeKind::TestOf => "test",
        EdgeKind::RouteRef => "route",
        EdgeKind::QueueEnqueue => "queue-enqueue",
        EdgeKind::QueueWorker => "queue-worker",
        EdgeKind::RouteTest => "route-test",
        EdgeKind::MarkdownLink => "md",
        EdgeKind::WorkspaceImport => "workspace",
        EdgeKind::CiInvocation => "ci",
        EdgeKind::HttpCall => "http",
        EdgeKind::ProcessSpawn => "process",
    }
}

// ── Shared serializable shape ────────────────────────────────────────────

#[derive(Serialize)]
#[serde(untagged)]
enum OutputNode {
    File(OutputFile),
    QueueJob(OutputQueueJob),
}

#[derive(Serialize)]
struct OutputFile {
    path: String,
    depth: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    via: Vec<&'static str>,
}

#[derive(Serialize)]
struct OutputQueueJob {
    #[serde(rename = "queueFile")]
    queue_file: String,
    job: String,
    depth: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    via: Vec<&'static str>,
}

#[derive(Serialize)]
struct Output {
    roots: Vec<String>,
    files: Vec<OutputNode>,
}

fn build_output(roots: &[String], entries: &[NodeEntry], root_dir: &Path) -> Output {
    Output {
        roots: roots.to_vec(),
        files: entries
            .iter()
            .map(|entry| {
                let via: Vec<&'static str> = entry.via.iter().map(|k| edge_kind_str(*k)).collect();
                match &entry.node {
                    NodeId::File(path) => {
                        let rel = path.strip_prefix(root_dir).unwrap_or(path);
                        OutputNode::File(OutputFile {
                            path: rel.to_string_lossy().into_owned(),
                            depth: entry.depth,
                            via,
                        })
                    }
                    NodeId::QueueJob { queue_file, job } => {
                        let rel = queue_file
                            .strip_prefix(root_dir)
                            .unwrap_or(queue_file.as_path());
                        OutputNode::QueueJob(OutputQueueJob {
                            queue_file: rel.to_string_lossy().into_owned(),
                            job: job.clone(),
                            depth: entry.depth,
                            via,
                        })
                    }
                }
            })
            .collect(),
    }
}

/// Write findings as a JSON object: `{ "roots": [...], "files": [...] }`.
pub fn write_json(
    roots: &[String],
    entries: &[NodeEntry],
    root_dir: &Path,
    w: &mut dyn Write,
) -> Result<()> {
    let out = build_output(roots, entries, root_dir);
    serde_json::to_writer_pretty(&mut *w, &out)?;
    writeln!(w)?;
    Ok(())
}

/// Write one relative path per line — suitable for shell `$()` substitution.
/// QueueJob virtual nodes are rendered as `queueFile#job`.
pub fn write_paths(entries: &[NodeEntry], root_dir: &Path, w: &mut dyn Write) -> Result<()> {
    for entry in entries {
        match &entry.node {
            NodeId::File(p) => {
                let rel = p.strip_prefix(root_dir).unwrap_or(p);
                writeln!(w, "{}", rel.display())?;
            }
            NodeId::QueueJob { queue_file, job } => {
                let rel = queue_file
                    .strip_prefix(root_dir)
                    .unwrap_or(queue_file.as_path());
                writeln!(w, "{}#{}", rel.display(), job)?;
            }
        }
    }
    Ok(())
}

/// Write a human-readable tree for TTY output.
pub fn write_human(
    roots: &[String],
    entries: &[NodeEntry],
    root_dir: &Path,
    w: &mut dyn Write,
) -> Result<()> {
    if roots.len() == 1 {
        writeln!(w, "{}", roots[0])?;
    } else {
        writeln!(w, "{} files", roots.len())?;
    }

    if entries.is_empty() {
        writeln!(w, "  (no results)")?;
        return Ok(());
    }

    for entry in entries {
        let name = entry.node.display_name(root_dir);
        let indent = "  ".repeat(entry.depth);
        writeln!(w, "{}{}", indent, name)?;
    }

    Ok(())
}

/// Write results as a Markdown nested bullet list.
pub fn write_md(
    roots: &[String],
    entries: &[NodeEntry],
    root_dir: &Path,
    w: &mut dyn Write,
) -> Result<()> {
    if roots.len() == 1 {
        writeln!(w, "# `{}`", roots[0])?;
    } else {
        writeln!(w, "# {} files", roots.len())?;
        for r in roots {
            writeln!(w, "- `{r}`")?;
        }
    }
    writeln!(w)?;

    if entries.is_empty() {
        writeln!(w, "_No results._")?;
        return Ok(());
    }

    for entry in entries {
        let name = entry.node.display_name(root_dir);
        let indent = "  ".repeat(entry.depth.saturating_sub(1));
        writeln!(w, "{}- `{}`", indent, name)?;
    }

    Ok(())
}

/// Write results as a YAML document with the same structure as JSON output.
pub fn write_yml(
    roots: &[String],
    entries: &[NodeEntry],
    root_dir: &Path,
    w: &mut dyn Write,
) -> Result<()> {
    let out = build_output(roots, entries, root_dir);
    let s = serde_yaml::to_string(&out)?;
    w.write_all(s.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests;
