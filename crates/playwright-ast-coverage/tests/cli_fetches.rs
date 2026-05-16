mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

fn with_fetches() -> std::path::PathBuf {
    common::fixture("nextjs-coverage", "with-fetches")
}

#[test]
fn edges_json_outputs_fetch_edges() {
    let output = Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(with_fetches())
        .arg("edges")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let edges = report["edges"].as_array().unwrap();
    let fetch_edges: Vec<_> = edges.iter().filter(|e| e["kind"] == "fetch").collect();
    assert!(!fetch_edges.is_empty(), "expected fetch edges");
    assert!(fetch_edges.iter().any(|e| {
        e["method"] == "GET"
            && e["path"] == "/api/health"
            && e["side"] == "server"
            && e["cached"] == false
            && e["route"] == "/"
    }));
}

#[test]
fn edges_json_fetch_edges_carry_test_identity() {
    let output = Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(with_fetches())
        .arg("edges")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let edges = report["edges"].as_array().unwrap();
    let named = edges
        .iter()
        .find(|e| e["kind"] == "fetch" && e["testName"] == "visits home page");
    assert!(
        named.is_some(),
        "expected fetch edge with testName=visits home page"
    );
    assert_eq!(named.unwrap()["describePath"][0], "Home");
    let bare = edges
        .iter()
        .find(|e| e["kind"] == "fetch" && e["testName"] == "also visits");
    assert!(
        bare.is_some(),
        "expected fetch edge with testName=also visits"
    );
}

#[test]
fn edges_text_outputs_fetch_edges() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(with_fetches())
        .arg("edges")
        .assert()
        .success()
        .stdout(predicate::str::contains("GET /api/health"));
}

#[test]
fn check_json_reports_fetch_coverage() {
    let output = Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(with_fetches())
        .arg("--json")
        .arg("check")
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["summary"]["totalFetchApis"], 1);
    assert_eq!(report["summary"]["coveredFetchApis"], 1);
    assert_eq!(report["summary"]["uncoveredFetchApis"], 0);
    let fetch_apis = report["fetchApis"].as_array().unwrap();
    assert!(!fetch_apis.is_empty());
    let api = &fetch_apis[0];
    assert_eq!(api["method"], "GET");
    assert_eq!(api["path"], "/api/health");
    assert_eq!(api["covered"], true);
    assert!(api["routeFiles"]
        .as_array()
        .unwrap()
        .contains(&Value::String("app/page.tsx".to_string())));
}

#[test]
fn check_json_fetch_tests_detail_carries_identity() {
    let output = Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(with_fetches())
        .arg("--json")
        .arg("check")
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let fetch_apis = report["fetchApis"].as_array().unwrap();
    let detail = fetch_apis[0]["testsDetail"].as_array().unwrap();
    assert!(detail.iter().any(|d| d["name"] == "visits home page"));
    assert!(detail.iter().any(|d| d["name"] == "also visits"));
}

#[test]
fn tests_json_reports_named_tests() {
    let output = Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(with_fetches())
        .arg("tests")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let tests = report["tests"].as_array().unwrap();
    // 3 entries: unnamed (dynamic name), "also visits", "visits home page"
    assert_eq!(tests.len(), 3);
    let visits_home = tests.iter().find(|t| t["name"] == "visits home page");
    assert!(visits_home.is_some());
    assert_eq!(visits_home.unwrap()["describePath"][0], "Home");
    assert!(visits_home.unwrap()["routes"]
        .as_array()
        .unwrap()
        .contains(&Value::String("/".to_string())));
    assert!(visits_home.unwrap()["fetchApis"]
        .as_array()
        .unwrap()
        .contains(&Value::String("GET /api/health".to_string())));
    let also_visits = tests.iter().find(|t| t["name"] == "also visits");
    assert!(also_visits.is_some());
    // unnamed entry (dynamic name test)
    let unnamed = tests.iter().find(|t| t["name"].is_null());
    assert!(unnamed.is_some());
}

#[test]
fn tests_text_reports_named_and_unnamed_tests() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(with_fetches())
        .arg("tests")
        .assert()
        .success()
        .stdout(predicate::str::contains("tests/home.spec.ts > also visits"))
        .stdout(predicate::str::contains(
            "tests/home.spec.ts > Home > visits home page",
        ))
        // unnamed test prints just the file path
        .stdout(predicate::str::contains("tests/home.spec.ts\n"))
        .stdout(predicate::str::contains("route: /"))
        .stdout(predicate::str::contains("fetch: GET /api/health"));
}

#[test]
fn tests_json_reports_test_ids_from_selectors() {
    let output = Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(common::fixture("nextjs-selectors", "selector-covered"))
        .arg("tests")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let tests = report["tests"].as_array().unwrap();
    let entry = tests.iter().find(|t| t["name"] == "covers selectors");
    assert!(entry.is_some());
    let test_ids = entry.unwrap()["testIds"].as_array().unwrap();
    assert!(test_ids.contains(&Value::String("save".to_string())));
    assert!(test_ids.contains(&Value::String("publish".to_string())));
}

#[test]
fn tests_text_reports_test_ids() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(common::fixture("nextjs-selectors", "selector-covered"))
        .arg("tests")
        .assert()
        .success()
        .stdout(predicate::str::contains("test-id: save"))
        .stdout(predicate::str::contains("test-id: publish"));
}

#[test]
fn tests_json_reports_html_ids() {
    let output = Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(common::fixture("nextjs-html-ids", "html-ids-covered"))
        .arg("tests")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let tests = report["tests"].as_array().unwrap();
    let entry = tests.iter().find(|t| t["name"] == "covers html ids");
    assert!(entry.is_some());
    let html_ids = entry.unwrap()["htmlIds"].as_array().unwrap();
    assert!(html_ids.contains(&Value::String("save".to_string())));
    assert!(html_ids.contains(&Value::String("publish".to_string())));
}

#[test]
fn tests_with_file_filter_excludes_nonmatching() {
    let output = Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(common::fixture("nextjs-selectors", "selector-covered"))
        .arg("tests")
        .arg("nonexistent.ts")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["tests"].as_array().unwrap().len(), 0);
}

#[test]
fn tests_with_fetch_file_filter_excludes_nonmatching() {
    // Exercises the Fetch edge filter continue in build_tests_report
    let output = Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(with_fetches())
        .arg("tests")
        .arg("nonexistent.ts")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["tests"].as_array().unwrap().len(), 0);
}

#[test]
fn tests_with_absolute_file_path_filter() {
    // Exercises the absolute path branch in input_file()
    let root = with_fetches();
    let abs_path = root.join("tests/home.spec.ts");
    let output = Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(&root)
        .arg("tests")
        .arg(abs_path)
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["tests"].as_array().unwrap().len(), 3);
}

#[test]
fn related_json_reports_fetch_apis() {
    let output = Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(with_fetches())
        .arg("related")
        .arg("app/page.tsx")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let fetch_apis = report["fetch_apis"].as_array().unwrap();
    assert!(fetch_apis.contains(&Value::String("GET /api/health".to_string())));
}
