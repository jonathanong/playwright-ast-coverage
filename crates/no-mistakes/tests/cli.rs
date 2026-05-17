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
