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
fn queues_edges_format_json_outputs_json() {
    let root = queue_fixture("basic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "json",
        "edges",
    ]);
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(json.as_array().is_some());
}

#[test]
fn queues_edges_format_paths_prints_one_per_line() {
    let root = queue_fixture("basic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "paths",
        "edges",
    ]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(!out.is_empty(), "paths output should be non-empty");
    for line in out.lines() {
        assert!(!line.contains("->"), "paths format should not contain ->");
    }
}

#[test]
fn queues_related_format_paths_prints_one_per_line() {
    let root = queue_fixture("basic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "paths",
        "related",
        "enqueue.ts",
    ]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(!out.is_empty(), "paths related output should be non-empty");
    for line in out.lines() {
        assert!(!line.contains("->"), "paths format should not contain ->");
    }
}

#[test]
fn queues_check_format_paths_prints_file_line() {
    let root = queue_fixture("dynamic");
    let output = run(&[
        "queues",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "paths",
        "check",
    ]);
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(!out.is_empty(), "paths check output should be non-empty");
    for line in out.lines() {
        assert!(
            line.contains(':'),
            "paths check line should be file:line format"
        );
        assert!(
            !line.contains("->"),
            "paths check line should not contain ->"
        );
    }
}

#[test]
fn server_routes_format_json_outputs_json() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "json",
        "routes",
    ]);
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(json.as_array().is_some());
}

#[test]
fn server_edges_format_paths_prints_one_per_line() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "paths",
        "edges",
    ]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(!out.is_empty(), "paths edges output should be non-empty");
    for line in out.lines() {
        assert!(!line.contains("->"), "paths format should not contain ->");
    }
}

#[test]
fn server_related_format_paths_prints_one_per_line() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "paths",
        "related",
        "backend/api/users.ts",
    ]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(!out.is_empty(), "paths related output should be non-empty");
    for line in out.lines() {
        assert!(!line.contains("->"), "paths format should not contain ->");
    }
}

#[test]
fn server_routes_format_paths_prints_files() {
    let root = server_fixture("express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "paths",
        "routes",
    ]);
    assert!(output.status.success());
    assert!(!stdout(&output).is_empty());
}

#[test]
fn react_analyze_format_json_outputs_json() {
    let root = react_fixture("react-traits-components", "basic");
    let output = run(&[
        "react",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "json",
        "analyze",
        "app/components/Greeting.tsx",
    ]);
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(json.as_array().is_some());
}

#[test]
fn react_check_format_json_exits_nonzero_on_violations() {
    let root = react_fixture("react-traits-config", "assert-no-fetch");
    let output = run(&[
        "react",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "json",
        "check",
        "--assert-no-fetch",
        "app/components/Fetcher.tsx",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(json.as_array().is_some());
}

#[test]
fn help_lists_check_subcommand() {
    let output = run(&["--help"]);
    assert!(output.status.success());
    let help = stdout(&output);
    assert!(help.contains("check"));
}

#[test]
fn global_check_passes_on_clean_queue_fixture() {
    let root = queue_fixture("basic");
    let output = run(&["check", "--root", root.to_str().unwrap()]);
    assert!(output.status.success());
}

#[test]
fn global_check_fails_on_bad_queue_fixture() {
    let root = queue_fixture("dynamic");
    let output = run(&["check", "--root", root.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn global_check_format_json_on_bad_queue_fixture() {
    let root = queue_fixture("dynamic");
    let output = run(&[
        "check",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(!output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(json.get("queues").and_then(|q| q.as_array()).is_some());
}

#[test]
fn queues_legacy_json_flag_still_works() {
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
    assert!(json.as_array().is_some());
}

#[test]
fn server_legacy_json_flag_still_works() {
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
    assert!(json.as_array().is_some());
}

#[test]
fn react_legacy_json_flag_still_works() {
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
fn react_analyze_human_format_succeeds() {
    let root = react_fixture("react-traits-components", "basic");
    let output = run(&[
        "react",
        "--root",
        root.to_str().unwrap(),
        "analyze",
        "app/components/Greeting.tsx",
    ]);
    assert!(output.status.success());
}

#[test]
fn react_check_human_format_violations_exits_nonzero() {
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
    assert!(!stdout(&output).is_empty());
}

#[test]
fn react_check_succeeds_when_no_violations() {
    let root = react_fixture("react-traits-components", "basic");
    let output = run(&[
        "react",
        "--root",
        root.to_str().unwrap(),
        "check",
        "--assert-no-fetch",
        "app/components/Greeting.tsx",
    ]);
    assert!(output.status.success());
}

#[test]
fn global_check_passes_on_react_fixture_with_no_violations() {
    let root = react_fixture("react-traits-components", "basic");
    let output = run(&["check", "--root", root.to_str().unwrap()]);
    assert!(output.status.success());
}

#[test]
fn global_check_relative_root_resolves() {
    let output = Command::new(bin())
        .current_dir(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .canonicalize()
                .unwrap(),
        )
        .args(["check", "--root", "fixtures/queue-ast-hop/basic"])
        .output()
        .expect("no-mistakes should run");
    assert!(output.status.success());
}

#[test]
fn react_analyze_relative_root_resolves() {
    let output = Command::new(bin())
        .current_dir(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .canonicalize()
                .unwrap(),
        )
        .args([
            "react",
            "--root",
            "fixtures/react-traits-components/basic",
            "analyze",
            "app/components/Greeting.tsx",
        ])
        .output()
        .expect("no-mistakes should run");
    assert!(output.status.success());
}
