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
fn test_cli_targets_imported_file() {
    let root = Path::new("tests/fixtures/targets-imported");
    fs::create_dir_all(root.join("app")).unwrap();
    fs::write(root.join(".no-mistakes.yaml"), "frontendRoot: app\n").unwrap();
    fs::write(
        root.join("app/page.tsx"),
        "
        import { getUsers } from './users';
        getUsers();
        ",
    )
    .unwrap();
    fs::write(
        root.join("app/users.ts"),
        "export const getUsers = () => fetch('/api/users');",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(root).arg("app/users.ts");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("### / (app/page.tsx)"))
        .stdout(predicate::str::contains("GET | `/api/users`"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_cli_target_match_modes() {
    let root = Path::new("tests/fixtures/targets-match-modes");
    fs::create_dir_all(root.join("app/users/profile")).unwrap();
    fs::write(root.join(".no-mistakes.yaml"), "frontendRoot: app\n").unwrap();
    fs::write(root.join("app/page.tsx"), "fetch('/api/root');").unwrap();
    fs::write(root.join("app/users/page.tsx"), "fetch('/api/users');").unwrap();
    fs::write(
        root.join("app/users/profile/page.tsx"),
        "fetch('/api/users-profile');",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root")
        .arg(root)
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

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_cli_route_handler_is_client_directive_ignored() {
    let root = Path::new("tests/fixtures/route-handler");
    fs::create_dir_all(root.join("app/api/hello")).unwrap();
    fs::write(root.join(".no-mistakes.yaml"), "frontendRoot: app\n").unwrap();
    fs::write(
        root.join("app/api/hello/route.ts"),
        "
        'use client';
        export async function GET() {
            return Response.json({});
        }
        fetch('/api/hello');
        ",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(root).arg("/api/hello");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "### /api/hello (app/api/hello/route.ts)",
        ))
        .stdout(predicate::str::contains("| GET | `/api/hello` |"))
        .stdout(predicate::str::contains(
            "| GET | `/api/hello` | server | app/api/hello/route.ts | 7 |",
        ))
        .stdout(predicate::str::contains("| no | ❌ |"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_cli_skips_type_only_imports() {
    let root = Path::new("tests/fixtures/type-only-import");
    fs::create_dir_all(root.join("app")).unwrap();
    fs::write(root.join(".no-mistakes.yaml"), "frontendRoot: app\n").unwrap();
    fs::write(
        root.join("app/page.tsx"),
        "
        import type { User } from './types';
        import { getData } from './runtime';
        export const user: User = {};
        getData();
        ",
    )
    .unwrap();
    fs::write(
        root.join("app/types.ts"),
        "export type User = {\n  id: string;\n};\nexport const getUser = () => fetch('/api/type-only');\n",
    )
    .unwrap();
    fs::write(
        root.join("app/runtime.ts"),
        "export const getData = () => fetch('/api/runtime');",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
    cmd.arg("--root").arg(root);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("GET | `/api/runtime`"))
        .stdout(predicate::str::contains("GET | `/api/type-only`").not());

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
        .stderr(predicate::str::contains(
            "Error: targets not found: [\"/missing\"]",
        ));

    fs::remove_dir_all(root).unwrap();
}
