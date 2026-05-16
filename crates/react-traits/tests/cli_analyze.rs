mod common;

use assert_cmd::Command;

fn cmd() -> Command {
    Command::cargo_bin("react-traits").unwrap()
}

#[test]
fn analyze_basic_greeting() {
    let root = common::fixture("react-traits-components", "basic");
    let output = cmd()
        .arg("analyze")
        .arg("app/components/Greeting.tsx")
        .arg("--root")
        .arg(&root)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(output).unwrap();
    assert!(
        out.contains("Greeting"),
        "expected Greeting in output: {out}"
    );
    assert!(
        out.contains("hasState: false"),
        "expected hasState: false in output: {out}"
    );
}

#[test]
fn analyze_counter_has_state() {
    let root = common::fixture("react-traits-components", "basic");
    let output = cmd()
        .arg("analyze")
        .arg("app/components/Counter.tsx")
        .arg("--root")
        .arg(&root)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(output).unwrap();
    assert!(
        out.contains("hasState: true"),
        "expected hasState: true in output: {out}"
    );
}

#[test]
fn analyze_json_output() {
    let root = common::fixture("react-traits-components", "basic");
    cmd()
        .arg("analyze")
        .arg("app/components/Greeting.tsx")
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .assert()
        .success();
}

#[test]
fn analyze_nested() {
    let root = common::fixture("react-traits-components", "nested");
    cmd()
        .arg("analyze")
        .arg("app/components/Parent.tsx")
        .arg("app/components/Child.tsx")
        .arg("--root")
        .arg(&root)
        .assert()
        .success();
}

#[test]
fn analyze_error_exit_code_on_failure() {
    // Passing an invalid config file path causes run_cli to return Err,
    // which exercises the error exit path in main.rs.
    cmd()
        .arg("analyze")
        .arg("**/*.tsx")
        .arg("--root")
        .arg(".")
        .arg("--config")
        .arg("/nonexistent/path/config.yaml")
        .assert()
        .code(2);
}
