mod common;

use assert_cmd::Command;

fn cmd() -> Command {
    Command::cargo_bin("react-traits").unwrap()
}

#[test]
fn check_assert_no_fetch_violation() {
    let root = common::fixture("react-traits-config", "assert-no-fetch");
    cmd()
        .arg("check")
        .arg("app/components/Fetcher.tsx")
        .arg("--root")
        .arg(&root)
        .assert()
        .failure();
}

#[test]
fn check_no_violation_without_fetch() {
    let root = common::fixture("react-traits-components", "basic");
    cmd()
        .arg("check")
        .arg("app/components/Greeting.tsx")
        .arg("--root")
        .arg(&root)
        .arg("--assert-no-fetch")
        .assert()
        .success();
}

#[test]
fn check_json_output_with_violations() {
    let root = common::fixture("react-traits-config", "assert-no-fetch");
    cmd()
        .arg("--json")
        .arg("check")
        .arg("app/components/Fetcher.tsx")
        .arg("--root")
        .arg(&root)
        .assert()
        .failure();
}

#[test]
fn check_format_paths_outputs_violation_files() {
    let root = common::fixture("react-traits-config", "assert-no-fetch");
    let output = cmd()
        .arg("--format")
        .arg("paths")
        .arg("check")
        .arg("app/components/Fetcher.tsx")
        .arg("--root")
        .arg(&root)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(output).unwrap();
    assert!(
        out.contains("Fetcher.tsx"),
        "expected violation file path in output: {out}"
    );
}

#[test]
fn check_format_markdown_outputs_violation_files() {
    let root = common::fixture("react-traits-config", "assert-no-fetch");
    let output = cmd()
        .arg("--format")
        .arg("md")
        .arg("check")
        .arg("app/components/Fetcher.tsx")
        .arg("--root")
        .arg(&root)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(output).unwrap();
    assert!(
        out.contains("# React trait violations") && out.contains("Fetcher.tsx"),
        "expected markdown violation output: {out}"
    );
}

#[test]
fn check_format_yaml_outputs_violation_files() {
    let root = common::fixture("react-traits-config", "assert-no-fetch");
    let output = cmd()
        .arg("--format")
        .arg("yml")
        .arg("check")
        .arg("app/components/Fetcher.tsx")
        .arg("--root")
        .arg(&root)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(output).unwrap();
    assert!(out.contains("file: app/components/Fetcher.tsx"));
}

#[test]
fn check_with_nonexistent_config_path_returns_error() {
    // Triggers the `?` error branch in run_check for this binary instantiation.
    let root = common::fixture("react-traits-config", "assert-no-fetch");
    cmd()
        .arg("--config")
        .arg("/nonexistent/config.yaml")
        .arg("check")
        .arg("app/components/Fetcher.tsx")
        .arg("--root")
        .arg(&root)
        .assert()
        .failure();
}
