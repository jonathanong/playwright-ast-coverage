use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn coverage_json_reports_uncovered_routes() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("uncovered"))
        .arg("--json")
        .assert()
        .code(1)
        .stdout(predicate::str::contains(r#""uncoveredRoutes": 1"#))
        .stdout(predicate::str::contains(r#""route": "/settings""#));
}

#[test]
fn ignored_routes_do_not_fail_coverage() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("ignored"))
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""uncoveredRoutes": 0"#));
}

#[test]
fn coverage_text_reports_all_routes_covered() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("covered"))
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "All routes and selectors covered.",
        ));
}

#[test]
fn relative_root_is_resolved() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .current_dir(fixture("covered"))
        .arg("--root")
        .arg(".")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "All routes and selectors covered.",
        ));
}

#[test]
fn duplicate_routes_and_selectors_are_sorted_deterministically() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("sort-tiebreakers"))
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""uncoveredRoutes": 0"#))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 0"#));
}

#[test]
fn coverage_text_reports_uncovered_routes() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("uncovered"))
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Uncovered routes:"))
        .stdout(predicate::str::contains("/settings"));
}

#[test]
fn edge_mode_migrates_exec_routetest_case() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("codebase-intel"))
        .arg("--mode")
        .arg("edges")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r#""testFile": "tests/e2e/users.spec.ts""#,
        ))
        .stdout(predicate::str::contains(
            r#""routeFile": "packages/web/app/users/[id]/page.tsx""#,
        ))
        .stdout(predicate::str::contains(r#""route": "/users/:id""#))
        .stdout(predicate::str::contains(r#""url": "/users/42""#));
}

#[test]
fn edge_text_outputs_edges() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("codebase-intel"))
        .arg("--mode")
        .arg("edges")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "tests/e2e/users.spec.ts -> packages/web/app/users/[id]/page.tsx",
        ));
}

#[test]
fn reads_playwright_config_by_default() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("playwright-config"))
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "All routes and selectors covered.",
        ));
}

#[test]
fn nonliteral_playwright_config_values_are_ignored_when_optional() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nonliteral-playwright-config"))
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "All routes and selectors covered.",
        ));
}

#[test]
fn navigation_helpers_cover_routes() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("navigation-helper"))
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""uncoveredRoutes": 0"#))
        .stdout(predicate::str::contains(r#""/users/42""#));
}

#[test]
fn scanner_edge_cases_are_covered_from_fixture() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("scanner-edge-cases"))
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""uncoveredRoutes": 0"#))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 0"#))
        .stdout(predicate::str::contains(r#""route": "/docs/*""#))
        .stdout(predicate::str::contains(r#""route": "/shop/**""#))
        .stdout(predicate::str::contains(r#""route": "/settings""#))
        .stdout(predicate::str::contains("string-example").not())
        .stdout(predicate::str::contains("line-comment").not())
        .stdout(predicate::str::contains("commented-line").not())
        .stdout(predicate::str::contains("//example.com/external").not());
}

#[test]
fn duplicate_slash_empty_segments_do_not_cover_dynamic_routes() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("empty-segment-route"))
        .arg("--json")
        .assert()
        .code(1)
        .stdout(predicate::str::contains(r#""uncoveredRoutes": 1"#))
        .stdout(predicate::str::contains(
            r#""route": "/users/:id/settings""#,
        ));
}

#[test]
fn selector_roots_and_excludes_are_configurable() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("selector-roots"))
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""totalSelectors": 1"#))
        .stdout(predicate::str::contains(r#""value": "save-button""#))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 0"#))
        .stdout(predicate::str::contains("ignored-test-selector").not());
}

#[test]
fn missing_explicit_config_exits_with_error() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("covered"))
        .arg("--config")
        .arg("missing.yaml")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("config file does not exist"));
}

#[test]
fn missing_playwright_config_exits_with_error() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("covered"))
        .arg("--playwright-config")
        .arg("missing-playwright.config.ts")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Playwright config does not exist"));
}

#[test]
fn missing_routes_exits_with_error() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("empty-app"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains("no Next.js page routes found"));
}

#[test]
fn absolute_urls_matching_base_url_cover_routes() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("base-url"))
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "All routes and selectors covered.",
        ));
}

#[test]
fn external_urls_without_base_url_are_ignored() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("external-url"))
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Uncovered routes:"));
}

#[test]
fn missing_project_test_dir_is_skipped() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("missing-test-dir"))
        .arg("--json")
        .assert()
        .code(1)
        .stdout(predicate::str::contains(r#""uncoveredRoutes": 1"#));
}

#[test]
fn invalid_project_config_exits_with_error() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("invalid-project-config"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains("expected string literal"));
}

#[test]
fn invalid_root_config_exits_with_error() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("invalid-root-config"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains("expected string literal"));
}

#[test]
fn selector_coverage_reports_all_selectors_covered() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("selector-covered"))
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""totalSelectors": 2"#))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 0"#))
        .stdout(predicate::str::contains(r#""value": "save""#))
        .stdout(predicate::str::contains(r#""value": "publish""#));
}

#[test]
fn selector_coverage_reports_uncovered_selectors() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("selector-uncovered"))
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Uncovered selectors:"))
        .stdout(predicate::str::contains(r#"[data-testid="save"]"#));
}

#[test]
fn selector_coverage_supports_fuzzy_templates() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("selector-fuzzy"))
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""totalSelectors": 2"#))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 0"#))
        .stdout(predicate::str::contains(r#""value": "user-${params.id}""#))
        .stdout(predicate::str::contains(r#""getByTestId(user-42)""#));
}

#[test]
fn selector_coverage_supports_custom_attributes() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("selector-custom"))
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""attribute": "data-test""#))
        .stdout(predicate::str::contains(r#""attribute": "data-test-id""#))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 0"#));
}

#[test]
fn selector_coverage_can_be_disabled() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("selector-disabled"))
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""totalSelectors": 0"#));
}

#[test]
fn selector_coverage_marks_unsupported_dynamic_values() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("selector-unsupported"))
        .arg("--json")
        .assert()
        .code(1)
        .stdout(predicate::str::contains(r#""unsupportedDynamic": true"#))
        .stdout(predicate::str::contains(r#""value": "{id}""#));
}

#[test]
fn get_by_test_id_uses_playwright_test_id_attribute() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("testid-attribute"))
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""attribute": "data-pw""#))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 0"#));
}

#[test]
fn edge_mode_outputs_selector_edges() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("selector-covered"))
        .arg("--mode")
        .arg("edges")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""kind": "selector""#))
        .stdout(predicate::str::contains(r#""appFile": "web/app/page.tsx""#))
        .stdout(predicate::str::contains(
            r#""selector": "getByTestId(save)""#,
        ));
}

#[test]
fn edge_text_outputs_selector_edges() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("selector-covered"))
        .arg("--mode")
        .arg("edges")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r#"tests/e2e/app.spec.ts -> web/app/page.tsx ([data-testid="save"], getByTestId(save))"#,
        ));
}
