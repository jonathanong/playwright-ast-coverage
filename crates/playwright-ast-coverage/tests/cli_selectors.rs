mod common;

use assert_cmd::Command;
use common::fixture;
use predicates::prelude::*;

#[test]
fn analysis_uses_only_matching_project_context() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-routes", "project-scoped-context"))
        .arg("--json")
        .arg("check")
        .assert()
        .code(1)
        .stdout(predicate::str::contains(r#""uncoveredRoutes": 1"#))
        .stdout(predicate::str::contains(r#""route": "/admin""#))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 1"#))
        .stdout(predicate::str::contains(r#""value": "home""#));
}

#[test]
fn edge_text_outputs_selector_edges() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-selectors", "selector-covered"))
        .arg("edges")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r#"tests/e2e/app.spec.ts -> web/app/page.tsx ([data-testid="save"], getByTestId(save))"#,
        ));
}

#[test]
fn yaml_test_exclude_skips_matching_test_files() {
    // Exercises the yaml_exclude `continue` branch in discover.rs.
    // The fixture has tests/skip/skipped.spec.ts excluded by testExclude in the yaml.
    let output = Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-selectors", "selector-test-exclude"))
        .arg("tests")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let tests = report["tests"].as_array().unwrap();
    // Only e2e/app.spec.ts should appear; skip/skipped.spec.ts must be excluded.
    assert!(
        tests
            .iter()
            .all(|t| !t["file"].as_str().unwrap_or("").contains("skipped.spec.ts")),
        "skipped.spec.ts should be excluded by testExclude"
    );
    assert!(tests
        .iter()
        .any(|t| t["file"].as_str().unwrap_or("").contains("e2e/app.spec.ts")));
}
