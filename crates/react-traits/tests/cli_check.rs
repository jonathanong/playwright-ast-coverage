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
