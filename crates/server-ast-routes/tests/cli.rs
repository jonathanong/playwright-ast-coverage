use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/server-ast-routes")
        .join(name)
}

#[test]
fn routes_json_reports_normalized_routes() {
    Command::cargo_bin("server-ast-routes")
        .unwrap()
        .arg("--root")
        .arg(fixture("express"))
        .arg("--json")
        .arg("routes")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""route": "/api/v1/users/*""#));
}

#[test]
fn routes_formats_are_rendered() {
    for format in ["md", "yml", "paths", "human"] {
        Command::cargo_bin("server-ast-routes")
            .unwrap()
            .arg("--root")
            .arg(fixture("mixed"))
            .arg("--format")
            .arg(format)
            .arg("routes")
            .arg("backend/api/routes.ts")
            .assert()
            .success()
            .stdout(predicate::str::contains("/api-server"));
    }
}

#[test]
fn edges_formats_are_rendered() {
    for format in ["json", "md", "yml", "human"] {
        Command::cargo_bin("server-ast-routes")
            .unwrap()
            .arg("--root")
            .arg(fixture("mixed"))
            .arg("--format")
            .arg(format)
            .arg("edges")
            .assert()
            .success()
            .stdout(predicate::str::contains("/matched"));
    }
}

#[test]
fn edges_paths_can_start_from_file() {
    Command::cargo_bin("server-ast-routes")
        .unwrap()
        .arg("--root")
        .arg(fixture("express"))
        .arg("--format")
        .arg("paths")
        .arg("--depth")
        .arg("1")
        .arg("edges")
        .arg("backend/api/users.ts")
        .assert()
        .success()
        .stdout(predicate::str::contains("/api/v1/users/*"));
}

#[test]
fn edges_json_honors_roots_and_depth() {
    Command::cargo_bin("server-ast-routes")
        .unwrap()
        .arg("--root")
        .arg(fixture("express"))
        .arg("--format")
        .arg("json")
        .arg("--depth")
        .arg("1")
        .arg("edges")
        .arg("backend/api/users.ts")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""roots": ["#))
        .stdout(predicate::str::contains("/api/v1/users/*"))
        .stdout(predicate::str::contains("/api/v1/ignored").not());
}

#[test]
fn edges_depth_walk_stops_after_leaf_frontier() {
    Command::cargo_bin("server-ast-routes")
        .unwrap()
        .arg("--root")
        .arg(fixture("express"))
        .arg("--format")
        .arg("json")
        .arg("--depth")
        .arg("2")
        .arg("edges")
        .arg("backend/api/users.ts")
        .assert()
        .success()
        .stdout(predicate::str::contains("/api/v1/users/*"));
}

#[test]
fn related_supports_directions_and_formats() {
    for format in ["json", "md", "yml", "human", "paths"] {
        Command::cargo_bin("server-ast-routes")
            .unwrap()
            .arg("--root")
            .arg(fixture("express"))
            .arg("--format")
            .arg(format)
            .arg("related")
            .arg("backend/api/users.ts")
            .arg("--direction")
            .arg("deps")
            .assert()
            .success()
            .stdout(predicate::str::contains("/api/v1/users"));
    }
}

#[test]
fn related_supports_dependents_and_both_directions() {
    for direction in ["dependents", "both"] {
        Command::cargo_bin("server-ast-routes")
            .unwrap()
            .arg("--root")
            .arg(fixture("express"))
            .arg("--format")
            .arg("json")
            .arg("related")
            .arg("backend/api/users.ts")
            .arg("--direction")
            .arg(direction)
            .assert()
            .success()
            .stdout(predicate::str::contains("edges"));
    }
}

#[test]
fn related_with_unmatched_root_stops_when_frontier_is_empty() {
    Command::cargo_bin("server-ast-routes")
        .unwrap()
        .arg("--root")
        .arg(fixture("express"))
        .arg("--format")
        .arg("json")
        .arg("related")
        .arg("backend/api/missing.ts")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""edges": []"#));
}

#[test]
fn invalid_filter_surfaces_main_error_exit() {
    Command::cargo_bin("server-ast-routes")
        .unwrap()
        .arg("--root")
        .arg(fixture("express"))
        .arg("--filter")
        .arg("[")
        .arg("routes")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("error:"));
}

#[test]
fn timings_and_jobs_env_are_accepted() {
    Command::cargo_bin("server-ast-routes")
        .unwrap()
        .env("RAYON_NUM_THREADS", "not-a-number")
        .arg("--root")
        .arg(fixture("hono"))
        .arg("--timings")
        .arg("-j")
        .arg("1")
        .arg("routes")
        .assert()
        .success()
        .stderr(predicate::str::contains("analysis:"));
}
