use std::path::PathBuf;
use std::process::{Command, Output};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_no-mistakes"))
}

fn fixture(name: &str) -> PathBuf {
    no_mistakes_core::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis")
            .join(name),
    )
}

fn react_fixture(category: &str, name: &str) -> PathBuf {
    no_mistakes_core::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(category)
            .join(name),
    )
}

fn run(args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("no-mistakes should run")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be utf8")
}

#[test]
fn dependencies_subcommand_outputs_transitive_imports() {
    let root = fixture("simple");
    let output = run(&[
        "dependencies",
        "a.mts",
        "--root",
        root.to_str().unwrap(),
        "--json",
    ]);

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    let paths: Vec<&str> = json["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| file["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths, vec!["b.mts", "c.mts"]);
}

#[test]
fn dependents_subcommand_outputs_reverse_imports() {
    let root = fixture("simple");
    let output = run(&[
        "dependents",
        "c.mts",
        "--root",
        root.to_str().unwrap(),
        "--json",
    ]);

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    let paths: Vec<&str> = json["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| file["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths, vec!["b.mts", "a.mts"]);
}

#[test]
fn symbols_subcommand_outputs_exports() {
    let root = fixture("simple");
    let output = run(&[
        "symbols",
        "a.mts",
        "--root",
        root.to_str().unwrap(),
        "--json",
    ]);

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    let exports = json["files"][0]["exports"].as_array().unwrap();
    assert_eq!(exports[0]["name"], "a");
}

#[test]
fn help_lists_scoped_subcommands() {
    let output = run(&["--help"]);

    assert!(output.status.success());
    let help = stdout(&output);
    assert!(help.contains("dependencies"));
    assert!(help.contains("dependents"));
    assert!(help.contains("symbols"));
    assert!(help.contains("react"));
    assert!(help.contains("queues"));
    assert!(help.contains("server"));
}

fn queue_fixture(name: &str) -> PathBuf {
    no_mistakes_core::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/queue-ast-hop")
            .join(name),
    )
}

fn server_fixture(name: &str) -> PathBuf {
    no_mistakes_core::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/server-ast-routes")
            .join(name),
    )
}

#[test]
fn queues_edges_json_reports_queue_job() {
    let root = queue_fixture("basic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "--json",
        "edges",
    ]);

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(json
        .as_array()
        .unwrap()
        .iter()
        .any(|e| { e["from"].as_str().unwrap_or("").contains("sendWelcome") }));
}

#[test]
fn queues_related_human_reports_related_files() {
    let root = queue_fixture("basic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "related",
        "enqueue.ts",
    ]);

    assert!(output.status.success());
    assert!(stdout(&output).contains("sendWelcome"));
}

#[test]
fn queues_check_fails_for_unmatched_worker() {
    let root = queue_fixture("dynamic");
    let output = run(&["queues", "--root", root.to_str().unwrap(), "check"]);

    assert!(!output.status.success());
    assert!(stdout(&output).contains("unmatched-worker"));
}

#[test]
fn server_routes_json_lists_routes() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "--json",
        "routes",
    ]);

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(json
        .as_array()
        .unwrap()
        .iter()
        .any(|r| { r["route"].as_str().unwrap_or("").contains("/api/v1/users") }));
}

#[test]
fn server_edges_human_shows_edges() {
    let root = server_fixture("express");
    let output = run(&["server", "--root", root.to_str().unwrap(), "edges"]);

    assert!(output.status.success());
    assert!(!stdout(&output).is_empty());
}

#[test]
fn server_related_json_shows_edges() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "--json",
        "related",
        "backend/api/users.ts",
    ]);

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(json.as_array().is_some());
}

#[test]
fn queues_edges_human_format_without_files() {
    let root = queue_fixture("basic");
    let output = run(&["queues", "--root", root.to_str().unwrap(), "edges"]);

    assert!(output.status.success());
    assert!(stdout(&output).contains("->"));
}

#[test]
fn queues_edges_with_specific_file() {
    let root = queue_fixture("basic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "edges",
        "enqueue.ts",
    ]);

    assert!(output.status.success());
    assert!(stdout(&output).contains("enqueue.ts ->"));
}

#[test]
fn queues_edges_json_with_specific_file() {
    let root = queue_fixture("basic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "--depth",
        "1",
        "--json",
        "edges",
        "enqueue.ts",
    ]);

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    let arr = json.as_array().expect("should be array");
    assert!(!arr.is_empty(), "should have edges from enqueue.ts");
    assert!(arr.iter().all(|e| e["from"].as_str() == Some("enqueue.ts")));
}

#[test]
fn queues_check_json_passes_for_clean_fixture() {
    let root = queue_fixture("basic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "--json",
        "check",
    ]);

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[test]
fn queues_related_json_shows_edges() {
    let root = queue_fixture("basic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "--json",
        "related",
        "enqueue.ts",
    ]);

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(json.as_array().is_some());
}

#[test]
fn queues_check_json_reports_findings() {
    let root = queue_fixture("dynamic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "--json",
        "check",
    ]);

    assert!(!output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(json.as_array().is_some());
}

#[test]
fn queues_check_passes_for_clean_fixture() {
    let root = queue_fixture("basic");
    let output = run(&["queues", "--root", root.to_str().unwrap(), "check"]);

    assert!(output.status.success());
}

#[test]
fn server_routes_human_format() {
    let root = server_fixture("express");
    let output = run(&["server", "--root", root.to_str().unwrap(), "routes"]);

    assert!(output.status.success());
    assert!(stdout(&output).contains("/api/v1/users"));
}

#[test]
fn server_routes_with_file_filter() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "routes",
        "backend/api/users.ts",
    ]);

    assert!(output.status.success());
    assert!(stdout(&output).contains("/api/v1/users"));
}

#[test]
fn server_edges_json_format() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "--json",
        "edges",
    ]);

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(json.as_array().is_some());
}

#[test]
fn server_edges_with_root_filter() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "edges",
        "backend/api/users.ts",
    ]);

    assert!(output.status.success());
}

#[test]
fn server_related_human_format() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "related",
        "backend/api/users.ts",
    ]);

    assert!(output.status.success());
}

#[test]
fn server_routes_json_with_file_filter() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "--json",
        "routes",
        "backend/api/users.ts",
    ]);

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(!json.as_array().unwrap().is_empty());
}

#[test]
fn server_edges_json_with_root_filter() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "--json",
        "edges",
        "backend/api/users.ts",
    ]);

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(json.as_array().is_some());
}

#[test]
fn queues_related_direction_deps() {
    let root = queue_fixture("basic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "related",
        "enqueue.ts",
        "--direction",
        "deps",
    ]);

    assert!(output.status.success());
}

#[test]
fn queues_related_direction_dependents() {
    let root = queue_fixture("basic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "related",
        "enqueue.ts",
        "--direction",
        "dependents",
    ]);

    assert!(output.status.success());
}

#[test]
fn server_related_direction_deps() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "related",
        "backend/api/users.ts",
        "--direction",
        "deps",
    ]);

    assert!(output.status.success());
}

#[test]
fn server_related_direction_dependents() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "related",
        "backend/api/users.ts",
        "--direction",
        "dependents",
    ]);

    assert!(output.status.success());
}

#[test]
fn server_relative_root_is_resolved() {
    let output = Command::new(bin())
        .current_dir(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .canonicalize()
                .unwrap(),
        )
        .args([
            "server",
            "--root",
            "fixtures/server-ast-routes/express",
            "routes",
        ])
        .output()
        .expect("no-mistakes should run");

    assert!(output.status.success());
}

#[test]
fn queues_relative_root_is_resolved() {
    let output = Command::new(bin())
        .current_dir(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .canonicalize()
                .unwrap(),
        )
        .args(["queues", "--root", "fixtures/queue-ast-hop/basic", "edges"])
        .output()
        .expect("no-mistakes should run");

    assert!(output.status.success());
}

#[test]
fn react_analyze_json_outputs_components() {
    let root = react_fixture("react-traits-components", "basic");
    let output = run(&[
        "react",
        "--root",
        root.to_str().unwrap(),
        "--json",
        "analyze",
        "app/components/Greeting.tsx",
    ]);
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(json.as_array().is_some());
}

#[test]
fn react_check_assert_no_fetch_exits_nonzero() {
    let root = react_fixture("react-traits-config", "assert-no-fetch");
    let output = run(&[
        "react",
        "--root",
        root.to_str().unwrap(),
        "check",
        "--assert-no-fetch",
        "app/components/Fetcher.tsx",
    ]);
    assert_eq!(output.status.code(), Some(1));
}
