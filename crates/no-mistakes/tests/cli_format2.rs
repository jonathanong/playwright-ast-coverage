use std::path::PathBuf;
use std::process::{Command, Output};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_no-mistakes"))
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
fn queues_edges_yml_format_outputs_yaml() {
    let root = queue_fixture("basic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "yml",
        "edges",
    ]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(!out.is_empty());
    assert!(out.contains("from:") || out.contains("sendWelcome"));
}

#[test]
fn queues_edges_md_format_outputs_markdown() {
    let root = queue_fixture("basic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "md",
        "edges",
    ]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("# Queue edges"));
}

#[test]
fn queues_related_yml_format_outputs_yaml() {
    let root = queue_fixture("basic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "yml",
        "related",
        "enqueue.ts",
    ]);
    assert!(output.status.success());
    assert!(!stdout(&output).is_empty());
}

#[test]
fn queues_related_md_format_outputs_markdown() {
    let root = queue_fixture("basic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "md",
        "related",
        "enqueue.ts",
    ]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("# Related queue files"));
}

#[test]
fn queues_check_yml_format_outputs_yaml() {
    let root = queue_fixture("dynamic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "yml",
        "check",
    ]);
    assert!(!output.status.success());
    assert!(!stdout(&output).is_empty());
}

#[test]
fn queues_check_md_format_outputs_markdown() {
    let root = queue_fixture("dynamic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "md",
        "check",
    ]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("# Queue check"));
}

#[test]
fn server_routes_yml_format_outputs_yaml() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "yml",
        "routes",
    ]);
    assert!(output.status.success());
    assert!(!stdout(&output).is_empty());
}

#[test]
fn server_routes_md_format_outputs_markdown() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "md",
        "routes",
    ]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("# Server routes"));
}

#[test]
fn server_routes_human_format_outputs_table() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "human",
        "routes",
    ]);
    assert!(output.status.success());
    assert!(!stdout(&output).is_empty());
}

#[test]
fn server_edges_yml_format_outputs_yaml() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "yml",
        "edges",
    ]);
    assert!(output.status.success());
    assert!(!stdout(&output).is_empty());
}

#[test]
fn server_edges_md_format_outputs_markdown() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "md",
        "edges",
    ]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("# Server route edges"));
}

#[test]
fn server_related_yml_format_outputs_yaml() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "yml",
        "related",
        "backend/api/users.ts",
    ]);
    assert!(output.status.success());
    assert!(!stdout(&output).is_empty());
}

#[test]
fn server_related_md_format_outputs_markdown() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "md",
        "related",
        "backend/api/users.ts",
    ]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("# Related server routes"));
}

#[test]
fn server_related_human_format_outputs_text() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "human",
        "related",
        "backend/api/users.ts",
    ]);
    assert!(output.status.success());
    assert!(!stdout(&output).is_empty());
}

#[test]
fn react_analyze_yml_format_outputs_yaml() {
    let root = react_fixture("react-traits-components", "basic");
    let output = run(&[
        "react",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "yml",
        "analyze",
        "app/components/Greeting.tsx",
    ]);
    assert!(output.status.success());
    assert!(!stdout(&output).is_empty());
}

#[test]
fn react_analyze_md_format_outputs_markdown() {
    let root = react_fixture("react-traits-components", "basic");
    let output = run(&[
        "react",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "md",
        "analyze",
        "app/components/Greeting.tsx",
    ]);
    assert!(output.status.success());
    assert!(!stdout(&output).is_empty());
}

#[test]
fn react_analyze_paths_format_outputs_paths() {
    let root = react_fixture("react-traits-components", "basic");
    let output = run(&[
        "react",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "paths",
        "analyze",
        "app/components/Greeting.tsx",
    ]);
    assert!(output.status.success());
    assert!(!stdout(&output).is_empty());
}

#[test]
fn react_check_yml_format_outputs_yaml_on_violation() {
    let root = react_fixture("react-traits-config", "assert-no-fetch");
    let output = run(&[
        "react",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "yml",
        "check",
        "--assert-no-fetch",
        "app/components/Fetcher.tsx",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(!stdout(&output).is_empty());
}

#[test]
fn react_check_md_format_outputs_markdown_on_violation() {
    let root = react_fixture("react-traits-config", "assert-no-fetch");
    let output = run(&[
        "react",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "md",
        "check",
        "--assert-no-fetch",
        "app/components/Fetcher.tsx",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(!stdout(&output).is_empty());
}

#[test]
fn react_check_paths_format_outputs_paths_on_violation() {
    let root = react_fixture("react-traits-config", "assert-no-fetch");
    let output = run(&[
        "react",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "paths",
        "check",
        "--assert-no-fetch",
        "app/components/Fetcher.tsx",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(!stdout(&output).is_empty());
}
