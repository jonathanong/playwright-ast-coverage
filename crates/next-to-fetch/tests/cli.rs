use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;

#[test]
fn test_cli_basic() {
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg("tests/fixtures/next-app");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Route: / (app/page.tsx)"))
        .stdout(predicate::str::contains("GET /api/home"))
        .stdout(predicate::str::contains(
            "Route: /users (app/users/page.tsx)",
        ))
        .stdout(predicate::str::contains("POST /api/users"));
}

#[test]
fn test_cli_json() {
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root")
        .arg("tests/fixtures/next-app")
        .arg("--json");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"route\": \"/\""))
        .stdout(predicate::str::contains("\"method\": \"POST\""));
}

#[test]
fn test_cli_missing_frontend_root() {
    let root = Path::new("tests/fixtures/missing-frontend");
    fs::create_dir_all(root).unwrap();
    fs::write(root.join(".no-mistakes.yaml"), "frontendRoot: missing\n").unwrap();

    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(root);
    cmd.assert().failure().stderr(predicate::str::contains(
        "frontend root directory does not exist",
    ));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_cli_missing_root() {
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg("non-existent-root");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("root directory does not exist"));
}

#[test]
fn test_cli_config_not_found() {
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--config").arg("non-existent.yaml");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("config file does not exist"));
}

#[test]
fn test_cli_custom_config() {
    let root = Path::new("tests/fixtures/custom-config");
    fs::create_dir_all(root.join("app")).unwrap();
    fs::write(root.join(".no-mistakes.yaml"), "frontendRoot: app\n").unwrap();
    fs::write(root.join("app/page.tsx"), "fetch('/api/custom')").unwrap();

    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(root);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("GET /api/custom"));

    fs::remove_dir_all(root).unwrap();
}
