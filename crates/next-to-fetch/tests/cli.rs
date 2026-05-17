mod common;

use assert_cmd::Command;
use common::fixture;
use predicates::prelude::*;

#[test]
fn test_cli_basic() {
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(fixture("nextjs-fetches", "next-app"));
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
        .arg(fixture("nextjs-fetches", "next-app"))
        .arg("--json");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"route\": \"/\""))
        .stdout(predicate::str::contains("\"method\": \"POST\""))
        .stdout(predicate::str::contains("\"summary\": {"));
}

#[test]
fn test_cli_format_paths() {
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root")
        .arg(fixture("nextjs-fetches", "next-app"))
        .arg("--format")
        .arg("paths");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("app/page.tsx"))
        .stdout(predicate::str::contains("### /").not());
}

#[test]
fn test_cli_format_yml_uses_structured_output() {
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root")
        .arg(fixture("nextjs-fetches", "next-app"))
        .arg("--format")
        .arg("yml");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("routes:"))
        .stdout(predicate::str::contains("summary:"))
        .stdout(predicate::str::contains("\"route\":").not());
}

#[test]
fn test_cli_missing_frontend_root() {
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root")
        .arg(fixture("nextjs-fetches", "missing-frontend"));
    cmd.assert().failure().stderr(predicate::str::contains(
        "frontend root directory does not exist",
    ));
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
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root")
        .arg(fixture("nextjs-fetches", "layout-traversal"));
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("GET | `/api/root-layout`"))
        .stdout(predicate::str::contains("GET | `/api/sub-layout`"))
        .stdout(predicate::str::contains("GET | `/api/page`"))
        .stdout(predicate::str::contains("## Duplicates"))
        .stdout(predicate::str::contains("GET | `/api/dup`"))
        .stdout(predicate::str::contains("## Unsupported (Dynamic)"))
        .stdout(predicate::str::contains("GET | `dynamic`"));
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
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root")
        .arg(fixture("nextjs-fetches", "custom-config"));
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("GET | `/api/custom`"));
}

#[test]
fn test_cli_targets() {
    let root = fixture("nextjs-fetches", "targets");
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(&root).arg("/users");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("GET | `/api/users`"))
        .stdout(predicate::str::contains("GET | `/api/root`").not());
}

#[test]
fn test_cli_targets_imported_file() {
    let root = fixture("nextjs-fetches", "targets-imported");
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(&root).arg("app/users.ts");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("### / (app/page.tsx)"))
        .stdout(predicate::str::contains("GET | `/api/users`"));
}

#[test]
fn test_cli_targets_imported_file_abs_path() {
    let root = fixture("nextjs-fetches", "targets-imported-abs");
    let target = root.join("app/users.ts").canonicalize().unwrap();
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(&root).arg(target);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("### / (app/page.tsx)"))
        .stdout(predicate::str::contains("GET | `/api/users`"));
}

#[test]
fn test_cli_target_file_match_uses_layout_direct_target() {
    let root = fixture("nextjs-fetches", "targets-layout-direct");
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(&root).arg("app/layout.tsx");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("### / (app/page.tsx)"))
        .stdout(predicate::str::contains("GET | `/api/page`"))
        .stdout(predicate::str::contains("GET | `/api/layout`"));
}

#[test]
fn test_cli_target_file_match_uses_layout_import_chain() {
    let root = fixture("nextjs-fetches", "targets-layout-import-chain");
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(&root).arg("app/shared.ts");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("### / (app/page.tsx)"))
        .stdout(predicate::str::contains("GET | `/api/page`"))
        .stdout(predicate::str::contains("GET | `/api/layout`"));
}

#[test]
fn test_cli_target_file_match_uses_layout_import_chain_transitively() {
    let root = fixture("nextjs-fetches", "targets-layout-import-chain-transitive");
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(&root).arg("app/target.ts");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("### / (app/page.tsx)"))
        .stdout(predicate::str::contains("GET | `/api/page`"))
        .stdout(predicate::str::contains("GET | `/api/layout`"));
}

#[test]
fn test_cli_target_file_match_uses_layout_import_chain_transitively_abs_path() {
    let root = fixture(
        "nextjs-fetches",
        "targets-layout-import-chain-transitive-abs",
    );
    let target = root.join("app/target.ts").canonicalize().unwrap();
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(&root).arg(target);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("### / (app/page.tsx)"))
        .stdout(predicate::str::contains("GET | `/api/page`"))
        .stdout(predicate::str::contains("GET | `/api/layout`"));
}

#[test]
fn test_cli_target_match_modes() {
    let root = fixture("nextjs-fetches", "targets-match-modes");
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root")
        .arg(&root)
        .args(["app/page.tsx", "/users/", "users/profile"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("### / (app/page.tsx)"))
        .stdout(predicate::str::contains("| GET | `/api/root` |"))
        .stdout(predicate::str::contains(
            "### /users/profile (app/users/profile/page.tsx)",
        ))
        .stdout(predicate::str::contains("| GET | `/api/users-profile` |"))
        .stdout(predicate::str::contains("| GET | `/api/users` |").not());
}

#[test]
fn test_cli_route_handler_is_client_directive_ignored() {
    let root = fixture("nextjs-fetches", "route-handler");
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(&root).arg("/api/hello");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "### /api/hello (app/api/hello/route.ts)",
        ))
        .stdout(predicate::str::contains("| GET | `/api/hello` |"))
        .stdout(predicate::str::contains("app/api/hello/route.ts | 5 |"))
        .stdout(predicate::str::contains("| no | ❌ |"));
}

#[test]
fn test_cli_skips_type_only_imports() {
    let root = fixture("nextjs-fetches", "type-only-import");
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(&root);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("GET | `/api/runtime`"))
        .stdout(predicate::str::contains("GET | `/api/type-only`").not());
}

#[test]
fn test_cli_targets_unmatched() {
    let root = fixture("nextjs-fetches", "targets-unmatched");
    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(&root).arg("/missing");
    cmd.assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains(
            "Error: targets not found: [\"/missing\"]",
        ));
}
