use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/queue-ast-hop")
        .join(name)
}

#[test]
fn edges_json_reports_virtual_queue_job() {
    Command::cargo_bin("queue-ast-hop")
        .unwrap()
        .arg("--root")
        .arg(fixture("basic"))
        .arg("--json")
        .arg("edges")
        .assert()
        .success()
        .stdout(predicate::str::contains("queues.ts#sendWelcome"));
}

#[test]
fn check_fails_for_unmatched_static_worker() {
    Command::cargo_bin("queue-ast-hop")
        .unwrap()
        .arg("--root")
        .arg(fixture("dynamic"))
        .arg("check")
        .assert()
        .failure()
        .stdout(predicate::str::contains("unmatched-worker"));
}

#[test]
fn related_paths_returns_worker() {
    Command::cargo_bin("queue-ast-hop")
        .unwrap()
        .arg("--root")
        .arg(fixture("basic"))
        .arg("--format")
        .arg("paths")
        .arg("related")
        .arg("enqueue.ts")
        .assert()
        .success()
        .stdout(predicate::str::contains("worker.ts"));
}

#[test]
fn check_json_reports_failures() {
    Command::cargo_bin("queue-ast-hop")
        .unwrap()
        .arg("--root")
        .arg(fixture("unmatched"))
        .arg("--json")
        .arg("check")
        .assert()
        .failure()
        .stdout(predicate::str::contains("unmatched-producer"));
}

#[test]
fn edges_formats_are_rendered() {
    for format in ["md", "yml", "paths", "human"] {
        Command::cargo_bin("queue-ast-hop")
            .unwrap()
            .arg("--root")
            .arg(fixture("basic"))
            .arg("--format")
            .arg(format)
            .arg("edges")
            .assert()
            .success()
            .stdout(predicate::str::contains("sendWelcome"));
    }
}

#[test]
fn related_supports_directions_and_json() {
    for direction in ["deps", "dependents", "both"] {
        Command::cargo_bin("queue-ast-hop")
            .unwrap()
            .arg("--root")
            .arg(fixture("basic"))
            .arg("--json")
            .arg("related")
            .arg("queues.ts#sendWelcome")
            .arg("--direction")
            .arg(direction)
            .assert()
            .success()
            .stdout(predicate::str::contains("edges"));
    }
}

#[test]
fn related_human_and_markdown_formats_render() {
    for format in ["human", "md", "yml"] {
        Command::cargo_bin("queue-ast-hop")
            .unwrap()
            .arg("--root")
            .arg(fixture("basic"))
            .arg("--format")
            .arg(format)
            .arg("related")
            .arg("enqueue.ts")
            .assert()
            .success()
            .stdout(predicate::str::contains("sendWelcome"));
    }
}

#[test]
fn timings_and_jobs_env_are_accepted() {
    Command::cargo_bin("queue-ast-hop")
        .unwrap()
        .env("RAYON_NUM_THREADS", "not-a-number")
        .arg("--root")
        .arg(fixture("basic"))
        .arg("--timings")
        .arg("edges")
        .assert()
        .success()
        .stderr(predicate::str::contains("search:"));
}

#[test]
fn relative_root_and_env_jobs_are_accepted() {
    Command::cargo_bin("queue-ast-hop")
        .unwrap()
        .current_dir(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .canonicalize()
                .unwrap(),
        )
        .env("RAYON_NUM_THREADS", "2")
        .arg("--root")
        .arg("fixtures/queue-ast-hop/basic")
        .arg("--format")
        .arg("human")
        .arg("edges")
        .arg("enqueue.ts")
        .assert()
        .success()
        .stdout(predicate::str::contains("enqueue.ts"));
}

#[test]
fn depth_limits_edges_from_roots() {
    Command::cargo_bin("queue-ast-hop")
        .unwrap()
        .arg("--root")
        .arg(fixture("basic"))
        .arg("--format")
        .arg("paths")
        .arg("--depth")
        .arg("1")
        .arg("edges")
        .arg("enqueue.ts")
        .assert()
        .success()
        .stdout(predicate::str::contains("queues.ts#sendWelcome"))
        .stdout(predicate::str::contains("worker.ts").not());
}

#[test]
fn check_failure_formats_are_rendered() {
    for format in ["md", "yml", "paths"] {
        Command::cargo_bin("queue-ast-hop")
            .unwrap()
            .arg("--root")
            .arg(fixture("unmatched"))
            .arg("--format")
            .arg(format)
            .arg("check")
            .assert()
            .failure()
            .stdout(predicate::str::contains("queues.ts"));
    }
}

#[test]
fn check_success_returns_success_exit_code() {
    Command::cargo_bin("queue-ast-hop")
        .unwrap()
        .arg("--root")
        .arg(fixture("basic"))
        .arg("check")
        .assert()
        .success();
}

#[test]
fn missing_tsconfig_surfaces_main_error_exit() {
    Command::cargo_bin("queue-ast-hop")
        .unwrap()
        .arg("--root")
        .arg(fixture("basic"))
        .arg("--tsconfig")
        .arg(fixture("basic/missing.json"))
        .arg("edges")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("error:"));
}
