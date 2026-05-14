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
        .stdout(predicate::str::contains("### / (app/page.tsx)"))
        .stdout(predicate::str::contains("| GET | `/api/home` |"))
        .stdout(predicate::str::contains("### /users (app/users/page.tsx)"))
        .stdout(predicate::str::contains("| POST | `/api/users` |"));
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
        .stdout(predicate::str::contains("\"method\": \"POST\""))
        .stdout(predicate::str::contains("\"summary\": {"));
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
fn test_cli_layout_traversal() {
    let root = Path::new("tests/fixtures/layout-traversal");
    fs::create_dir_all(root.join("app/sub")).unwrap();
    fs::write(root.join(".no-mistakes.yaml"), "frontendRoot: app\n").unwrap();
    fs::write(
        root.join("app/layout.tsx"),
        "fetch('/api/root-layout'); fetch('/api/dup')",
    )
    .unwrap();
    fs::write(
        root.join("app/sub/layout.tsx"),
        "fetch('/api/sub-layout'); fetch('/api/dup')",
    )
    .unwrap();
    fs::write(
        root.join("app/sub/page.tsx"),
        "fetch('/api/page'); fetch(url)",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(root);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("GET | `/api/root-layout`"))
        .stdout(predicate::str::contains("GET | `/api/sub-layout`"))
        .stdout(predicate::str::contains("GET | `/api/page`"))
        .stdout(predicate::str::contains("## Duplicates"))
        .stdout(predicate::str::contains("GET | `/api/dup`"))
        .stdout(predicate::str::contains("## Unsupported (Dynamic)"))
        .stdout(predicate::str::contains("GET | `dynamic`"));

    fs::remove_dir_all(root).unwrap();
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
        .stdout(predicate::str::contains("GET | `/api/custom`"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_cli_targets() {
    let root = Path::new("tests/fixtures/targets");
    fs::create_dir_all(root.join("app/users")).unwrap();
    fs::write(root.join(".no-mistakes.yaml"), "frontendRoot: app\n").unwrap();
    fs::write(root.join("app/layout.tsx"), "fetch('/api/layout')").unwrap();
    fs::write(root.join("app/page.tsx"), "fetch('/api/root')").unwrap();
    fs::write(root.join("app/users/page.tsx"), "fetch('/api/users')").unwrap();

    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(root).arg("/users");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("GET | `/api/users`"))
        .stdout(predicate::str::contains("GET | `/api/root`").not());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_cli_targets_unmatched() {
    let root = Path::new("tests/fixtures/targets-unmatched");
    fs::create_dir_all(root.join("app")).unwrap();
    fs::write(root.join(".no-mistakes.yaml"), "frontendRoot: app\n").unwrap();
    fs::write(root.join("app/page.tsx"), "fetch('/api/root')").unwrap();

    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(root).arg("/missing");
    cmd.assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("Error: targets not found: [\"/missing\"]"));

    fs::remove_dir_all(root).unwrap();
}
