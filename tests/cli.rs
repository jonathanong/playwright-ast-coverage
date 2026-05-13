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
        .arg("check")
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
        .arg("check")
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
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "All routes and selectors covered.",
        ));
}

#[test]
fn check_subcommand_reports_all_routes_covered() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("covered"))
        .arg("check")
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
        .arg("check")
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
        .arg("check")
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
        .arg("check")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Uncovered routes:"))
        .stdout(predicate::str::contains("/settings"));
}

#[test]
fn edges_json_outputs_route_edges() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("codebase-intel"))
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
        .arg("edges")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "tests/e2e/users.spec.ts -> packages/web/app/users/[id]/page.tsx",
        ));
}

#[test]
fn edges_subcommand_outputs_edges() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("codebase-intel"))
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
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "All routes and selectors covered.",
        ));
}

#[test]
fn default_discovery_reads_multiple_named_playwright_configs() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("multi-config"))
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "All routes and selectors covered.",
        ));
}

#[test]
fn project_filters_by_playwright_config_name() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("multi-config"))
        .arg("--project")
        .arg("storybook")
        .arg("related")
        .arg("web/app/users/[id]/page.tsx")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "playwright/storybook/user.stories.spec.ts",
        ))
        .stdout(predicate::str::contains("playwright/tests/home.spec.ts").not());
}

#[test]
fn project_filter_ignores_inner_playwright_project_names() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("multi-config"))
        .arg("--project")
        .arg("chromium")
        .arg("related")
        .arg("web/app/page.tsx")
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "no Playwright config found with name chromium",
        ));
}

#[test]
fn multi_config_requires_top_level_names() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("multi-config-missing-name"))
        .arg("check")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("must define top-level name"));
}

#[test]
fn multi_config_requires_unique_top_level_names() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("multi-config-duplicate-name"))
        .arg("check")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("is duplicated"));
}

#[test]
fn related_json_returns_direct_edge_tests() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("multi-config"))
        .arg("--json")
        .arg("related")
        .arg("web/app/users/[id]/page.tsx")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r#""tests": [
    "playwright/storybook/user.stories.spec.ts"
  ]"#,
        ));
}

#[test]
fn related_requires_at_least_one_file() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("multi-config"))
        .arg("related")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("required"));
}

#[test]
fn related_normalizes_dot_relative_paths() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("multi-config"))
        .arg("related")
        .arg("./web/app/users/[id]/page.tsx")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "playwright/storybook/user.stories.spec.ts",
        ));
}

#[test]
fn nonliteral_playwright_config_values_are_ignored_when_optional() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nonliteral-playwright-config"))
        .arg("check")
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
        .arg("check")
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
        .arg("check")
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
        .arg("check")
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
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""totalSelectors": 1"#))
        .stdout(predicate::str::contains(r#""value": "save-button""#))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 0"#))
        .stdout(predicate::str::contains("ignored-test-selector").not());
}

#[test]
fn skipped_tests_do_not_cover_by_default_but_conditional_tests_do() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("skipped-tests"))
        .arg("--json")
        .arg("check")
        .assert()
        .code(1)
        .stdout(predicate::str::contains(r#""uncoveredRoutes": 1"#))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 1"#))
        .stdout(predicate::str::contains(r#""route": "/skipped""#))
        .stdout(predicate::str::contains(
            "\"route\": \"/conditional\",\n      \"file\": \"web/app/conditional/page.tsx\",\n      \"covered\": true",
        ));
}

#[test]
fn allow_skipped_tests_counts_skipped_coverage() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("skipped-tests"))
        .arg("--allow-skipped-tests")
        .arg("--json")
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""uncoveredRoutes": 0"#))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 0"#));
}

#[test]
fn assert_conditional_tests_requires_active_coverage() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("skipped-tests"))
        .arg("--assert-conditional-tests")
        .arg("--json")
        .arg("check")
        .assert()
        .code(1)
        .stdout(predicate::str::contains(r#""uncoveredRoutes": 2"#))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 2"#));
}

#[test]
fn missing_explicit_config_exits_with_error() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("covered"))
        .arg("--config")
        .arg("missing.yaml")
        .arg("check")
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
        .arg("check")
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
        .arg("check")
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
        .arg("check")
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
        .arg("check")
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
        .arg("check")
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
        .arg("check")
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
        .arg("check")
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
        .arg("check")
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
        .arg("check")
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
        .arg("check")
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
        .arg("check")
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
        .arg("check")
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
        .arg("check")
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
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""attribute": "data-pw""#))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 0"#));
}

#[test]
fn analysis_uses_only_matching_project_context() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("project-scoped-context"))
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
fn edges_json_outputs_selector_edges() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("selector-covered"))
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
        .arg("edges")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r#"tests/e2e/app.spec.ts -> web/app/page.tsx ([data-testid="save"], getByTestId(save))"#,
        ));
}
