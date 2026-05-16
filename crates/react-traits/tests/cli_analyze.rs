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
