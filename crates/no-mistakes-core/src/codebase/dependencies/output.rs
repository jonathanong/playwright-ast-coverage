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

#[derive(Serialize)]
#[serde(untagged)]
enum JsonNode<'a> {
    File(JsonFile<'a>),
    QueueJob(JsonQueueJob<'a>),
}

#[derive(Serialize)]
struct JsonFile<'a> {
    path: &'a str,
    depth: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    via: Vec<&'static str>,
}

#[derive(Serialize)]
struct JsonQueueJob<'a> {
    #[serde(rename = "queueFile")]
    queue_file: &'a str,
    job: &'a str,
    depth: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    via: Vec<&'static str>,
}

#[derive(Serialize)]
struct JsonOutput<'a> {
    roots: Vec<&'a str>,
    files: Vec<JsonNode<'a>>,
}

/// Write findings as a JSON object: `{ "roots": [...], "files": [...] }`.
pub fn write_json(
    roots: &[String],
    entries: &[NodeEntry],
    root_dir: &Path,
    w: &mut impl Write,
) -> Result<()> {
    let root_strs: Vec<&str> = roots.iter().map(String::as_str).collect();

    let mut path_strs: Vec<String> = Vec::new();
    let mut queue_file_strs: Vec<String> = Vec::new();
    let mut job_strs: Vec<String> = Vec::new();
    for e in entries {
        match &e.node {
            NodeId::File(p) => {
                let rel = p.strip_prefix(root_dir).unwrap_or(p);
                path_strs.push(rel.to_string_lossy().into_owned());
                queue_file_strs.push(String::new());
                job_strs.push(String::new());
            }
            NodeId::QueueJob { queue_file, job } => {
                let rel = queue_file
                    .strip_prefix(root_dir)
                    .unwrap_or(queue_file.as_path());
                path_strs.push(String::new());
                queue_file_strs.push(rel.to_string_lossy().into_owned());
                job_strs.push(job.clone());
            }
        }
    }

    let files: Vec<JsonNode> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let via: Vec<&'static str> = e.via.iter().map(|k| edge_kind_str(*k)).collect();
            match &e.node {
                NodeId::File(_) => JsonNode::File(JsonFile {
                    path: &path_strs[i],
                    depth: e.depth,
                    via,
                }),
                NodeId::QueueJob { .. } => JsonNode::QueueJob(JsonQueueJob {
                    queue_file: &queue_file_strs[i],
                    job: &job_strs[i],
                    depth: e.depth,
                    via,
                }),
            }
        })
        .collect();

    let out = JsonOutput {
        roots: root_strs,
        files,
    };
    serde_json::to_writer_pretty(&mut *w, &out)?;
    writeln!(w)?;
    Ok(())
}

/// Write one relative path per line — suitable for shell `$()` substitution.
/// QueueJob virtual nodes are rendered as `queueFile#job`.
pub fn write_paths(entries: &[NodeEntry], root_dir: &Path, w: &mut impl Write) -> Result<()> {
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
    w: &mut impl Write,
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
    w: &mut impl Write,
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
    w: &mut impl Write,
) -> Result<()> {
    #[derive(Serialize)]
    #[serde(untagged)]
    enum YmlNode {
        File(YmlFile),
        QueueJob(YmlQueueJob),
    }
    #[derive(Serialize)]
    struct YmlFile {
        path: String,
        depth: usize,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        via: Vec<String>,
    }
    #[derive(Serialize)]
    struct YmlQueueJob {
        #[serde(rename = "queueFile")]
        queue_file: String,
        job: String,
        depth: usize,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        via: Vec<String>,
    }
    #[derive(Serialize)]
    struct YmlOutput {
        roots: Vec<String>,
        files: Vec<YmlNode>,
    }

    let root_strs: Vec<String> = roots.to_vec();
    let files: Vec<YmlNode> = entries
        .iter()
        .map(|e| {
            let via: Vec<String> = e
                .via
                .iter()
                .map(|k| edge_kind_str(*k).to_string())
                .collect();
            match &e.node {
                NodeId::File(p) => {
                    let rel = p.strip_prefix(root_dir).unwrap_or(p);
                    YmlNode::File(YmlFile {
                        path: rel.to_string_lossy().into_owned(),
                        depth: e.depth,
                        via,
                    })
                }
                NodeId::QueueJob { queue_file, job } => {
                    let rel = queue_file
                        .strip_prefix(root_dir)
                        .unwrap_or(queue_file.as_path());
                    YmlNode::QueueJob(YmlQueueJob {
                        queue_file: rel.to_string_lossy().into_owned(),
                        job: job.clone(),
                        depth: e.depth,
                        via,
                    })
                }
            }
        })
        .collect();

    let out = YmlOutput {
        roots: root_strs,
        files,
    };
    let s = serde_yaml::to_string(&out)?;
    w.write_all(s.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests;
