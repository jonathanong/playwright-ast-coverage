use super::*;
use std::path::{Path, PathBuf};

fn json_value(roots: &[String], entries: &[NodeEntry], root: &Path) -> serde_json::Value {
    let mut buf = Vec::new();
    write_json(roots, entries, root, &mut buf).unwrap();
    serde_json::from_slice(&buf).unwrap()
}

fn p(s: &str) -> PathBuf {
    PathBuf::from(s)
}

fn entry(path: &str, depth: usize) -> NodeEntry {
    NodeEntry {
        node: NodeId::File(p(path)),
        depth,
        via: vec![],
    }
}

fn entry_with_via(path: &str, depth: usize, via: Vec<EdgeKind>) -> NodeEntry {
    NodeEntry {
        node: NodeId::File(p(path)),
        depth,
        via,
    }
}

fn queue_job_entry(queue_file: &str, job: &str, depth: usize) -> NodeEntry {
    NodeEntry {
        node: NodeId::QueueJob {
            queue_file: p(queue_file),
            job: job.to_string(),
        },
        depth,
        via: vec![],
    }
}

// ── write_json ──────────────────────────────────────────────────────────

#[test]
fn json_empty_entries() {
    let root = p("/root");
    let v = json_value(&["src/a.mts".to_string()], &[], &root);
    assert_eq!(
        v,
        serde_json::json!({
            "roots": ["src/a.mts"],
            "files": [],
        })
    );
}

#[test]
fn json_with_entries_relative_paths() {
    let root = p("/root");
    let entries = vec![entry("/root/src/b.mts", 1), entry("/root/src/c.mts", 2)];
    let v = json_value(&["src/a.mts".to_string()], &entries, &root);
    assert_eq!(
        v,
        serde_json::json!({
            "roots": ["src/a.mts"],
            "files": [
                {"path": "src/b.mts", "depth": 1},
                {"path": "src/c.mts", "depth": 2},
            ],
        })
    );
}

#[test]
fn json_multiple_roots() {
    let root = p("/root");
    let v = json_value(&["a.mts".to_string(), "b.mts".to_string()], &[], &root);
    assert_eq!(
        v,
        serde_json::json!({
            "roots": ["a.mts", "b.mts"],
            "files": [],
        })
    );
}

#[test]
fn json_queue_job_node() {
    let root = p("/root");
    let entries = vec![queue_job_entry("/root/src/queues.mts", "sendWelcome", 1)];
    let v = json_value(&["src/enqueues.mts".to_string()], &entries, &root);
    assert_eq!(
        v,
        serde_json::json!({
            "roots": ["src/enqueues.mts"],
            "files": [
                {"queueFile": "src/queues.mts", "job": "sendWelcome", "depth": 1},
            ],
        })
    );
}

// ── write_paths ─────────────────────────────────────────────────────────

#[test]
fn paths_empty_entries() {
    let root = p("/root");
    let mut buf = Vec::new();
    write_paths(&[], &root, &mut buf).unwrap();
    assert!(buf.is_empty());
}

#[test]
fn paths_relative_output() {
    let root = p("/root");
    let entries = vec![entry("/root/src/b.mts", 1), entry("/root/src/c.mts", 2)];
    let mut buf = Vec::new();
    write_paths(&entries, &root, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert_eq!(s, "src/b.mts\nsrc/c.mts\n");
}

#[test]
fn paths_queue_job_rendered_as_hash() {
    let root = p("/root");
    let entries = vec![queue_job_entry("/root/src/queues.mts", "sendWelcome", 1)];
    let mut buf = Vec::new();
    write_paths(&entries, &root, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert_eq!(s, "src/queues.mts#sendWelcome\n");
}

// ── write_human ─────────────────────────────────────────────────────────

#[test]
fn human_no_entries() {
    let root = p("/root");
    let mut buf = Vec::new();
    write_human(&["src/a.mts".to_string()], &[], &root, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("(no results)"));
}

#[test]
fn human_with_entries_indented() {
    let root = p("/root");
    let entries = vec![entry("/root/b.mts", 1), entry("/root/c.mts", 2)];
    let mut buf = Vec::new();
    write_human(&["a.mts".to_string()], &entries, &root, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("  b.mts"), "depth-1 has 2-space indent");
    assert!(s.contains("    c.mts"), "depth-2 has 4-space indent");
}

#[test]
fn human_queue_job_rendered() {
    let root = p("/root");
    let entries = vec![queue_job_entry("/root/src/queues.mts", "sendWelcome", 1)];
    let mut buf = Vec::new();
    write_human(&["a.mts".to_string()], &entries, &root, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("src/queues.mts#sendWelcome"));
}

// ── write_md ─────────────────────────────────────────────────────────────

#[test]
fn md_empty_entries() {
    let root = p("/root");
    let mut buf = Vec::new();
    write_md(&["src/a.mts".to_string()], &[], &root, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("# `src/a.mts`"));
    assert!(s.contains("_No results._"));
}

#[test]
fn md_single_root_with_entries() {
    let root = p("/root");
    let entries = vec![entry("/root/b.mts", 1), entry("/root/c.mts", 2)];
    let mut buf = Vec::new();
    write_md(&["a.mts".to_string()], &entries, &root, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("# `a.mts`"));
    assert!(s.contains("- `b.mts`"));
    assert!(s.contains("  - `c.mts`")); // depth-2 → 2-space indent
}

#[test]
fn md_multiple_roots() {
    let root = p("/root");
    let mut buf = Vec::new();
    write_md(
        &["a.mts".to_string(), "b.mts".to_string()],
        &[],
        &root,
        &mut buf,
    )
    .unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("# 2 files"));
    assert!(s.contains("- `a.mts`"));
    assert!(s.contains("- `b.mts`"));
}

// ── write_yml ─────────────────────────────────────────────────────────────

#[test]
fn yml_empty_entries() {
    let root = p("/root");
    let mut buf = Vec::new();
    write_yml(&["src/a.mts".to_string()], &[], &root, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    let v: serde_yaml::Value = serde_yaml::from_str(&s).unwrap();
    assert_eq!(v["roots"].as_sequence().unwrap().len(), 1);
    assert_eq!(v["files"].as_sequence().unwrap().len(), 0);
}

#[test]
fn yml_with_entries() {
    let root = p("/root");
    let entries = vec![entry("/root/src/b.mts", 1)];
    let mut buf = Vec::new();
    write_yml(&["src/a.mts".to_string()], &entries, &root, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    let v: serde_yaml::Value = serde_yaml::from_str(&s).unwrap();
    let files = v["files"].as_sequence().unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0]["path"].as_str().unwrap(), "src/b.mts");
    assert_eq!(files[0]["depth"].as_u64().unwrap(), 1);
}

#[test]
fn yml_multiple_roots() {
    let root = p("/root");
    let mut buf = Vec::new();
    write_yml(
        &["a.mts".to_string(), "b.mts".to_string()],
        &[],
        &root,
        &mut buf,
    )
    .unwrap();
    let s = String::from_utf8(buf).unwrap();
    let v: serde_yaml::Value = serde_yaml::from_str(&s).unwrap();
    assert_eq!(v["roots"].as_sequence().unwrap().len(), 2);
}

#[test]
fn yml_depth_preserved() {
    let root = p("/root");
    let entries = vec![entry("/root/a.mts", 1), entry("/root/b.mts", 3)];
    let mut buf = Vec::new();
    write_yml(&["root.mts".to_string()], &entries, &root, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    let v: serde_yaml::Value = serde_yaml::from_str(&s).unwrap();
    let files = v["files"].as_sequence().unwrap();
    assert_eq!(files[1]["depth"].as_u64().unwrap(), 3);
}

#[test]
fn yml_queue_job_node() {
    let root = p("/root");
    let entries = vec![queue_job_entry("/root/src/queues.mts", "sendWelcome", 2)];
    let mut buf = Vec::new();
    write_yml(&["src/enqueues.mts".to_string()], &entries, &root, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    let v: serde_yaml::Value = serde_yaml::from_str(&s).unwrap();
    let files = v["files"].as_sequence().unwrap();
    assert_eq!(files[0]["queueFile"].as_str().unwrap(), "src/queues.mts");
    assert_eq!(files[0]["job"].as_str().unwrap(), "sendWelcome");
}

// ── via field in JSON/YAML output ────────────────────────────────────────

#[test]
fn json_via_empty_omitted() {
    let root = p("/root");
    let entries = vec![entry("/root/b.mts", 1)];
    let mut buf = Vec::new();
    write_json(&["a.mts".to_string()], &entries, &root, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    // via is omitted when empty
    assert!(
        v["files"][0].get("via").is_none()
            || v["files"][0]["via"]
                .as_array()
                .map(|a| a.is_empty())
                .unwrap_or(false)
    );
}

#[test]
fn json_via_included_when_present() {
    let root = p("/root");
    let entries = vec![entry_with_via(
        "/root/b.mts",
        1,
        vec![EdgeKind::Import, EdgeKind::TestOf],
    )];
    let mut buf = Vec::new();
    write_json(&["a.mts".to_string()], &entries, &root, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    let via = v["files"][0]["via"].as_array().unwrap();
    let via_strs: Vec<&str> = via.iter().map(|x| x.as_str().unwrap()).collect();
    assert!(via_strs.contains(&"import"));
    assert!(via_strs.contains(&"test"));
}

#[test]
fn yml_via_included_when_present() {
    let root = p("/root");
    let entries = vec![entry_with_via("/root/b.mts", 1, vec![EdgeKind::RouteRef])];
    let mut buf = Vec::new();
    write_yml(&["a.mts".to_string()], &entries, &root, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    let v: serde_yaml::Value = serde_yaml::from_str(&s).unwrap();
    let via = v["files"][0]["via"].as_sequence().unwrap();
    assert_eq!(via[0].as_str().unwrap(), "route");
}

#[test]
fn edge_kind_str_all_variants() {
    assert_eq!(edge_kind_str(EdgeKind::Import), "import");
    assert_eq!(edge_kind_str(EdgeKind::TypeImport), "type-import");
    assert_eq!(edge_kind_str(EdgeKind::DynamicImport), "dynamic-import");
    assert_eq!(edge_kind_str(EdgeKind::Require), "require");
    assert_eq!(edge_kind_str(EdgeKind::TestOf), "test");
    assert_eq!(edge_kind_str(EdgeKind::RouteRef), "route");
    assert_eq!(edge_kind_str(EdgeKind::QueueEnqueue), "queue-enqueue");
    assert_eq!(edge_kind_str(EdgeKind::QueueWorker), "queue-worker");
    assert_eq!(edge_kind_str(EdgeKind::RouteTest), "route-test");
    assert_eq!(edge_kind_str(EdgeKind::MarkdownLink), "md");
    assert_eq!(edge_kind_str(EdgeKind::WorkspaceImport), "workspace");
    assert_eq!(edge_kind_str(EdgeKind::CiInvocation), "ci");
    assert_eq!(edge_kind_str(EdgeKind::HttpCall), "http");
    assert_eq!(edge_kind_str(EdgeKind::ProcessSpawn), "process");
}
