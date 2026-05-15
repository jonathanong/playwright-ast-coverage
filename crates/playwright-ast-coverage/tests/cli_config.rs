mod common;

use assert_cmd::Command;
use common::fixture;
use predicates::prelude::*;

#[test]
fn reads_playwright_config_by_default() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("playwright-configs", "playwright-config"))
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
        .arg(fixture("playwright-configs", "multi-config"))
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
        .arg(fixture("playwright-configs", "multi-config"))
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
        .arg(fixture("playwright-configs", "multi-config"))
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
        .arg(fixture("playwright-configs", "multi-config-missing-name"))
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
        .arg(fixture("playwright-configs", "multi-config-duplicate-name"))
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
        .arg(fixture("playwright-configs", "multi-config"))
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
        .arg(fixture("playwright-configs", "multi-config"))
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
        .arg(fixture("playwright-configs", "multi-config"))
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
        .arg(fixture(
            "playwright-configs",
            "nonliteral-playwright-config",
        ))
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
        .arg(fixture("nextjs-routes", "navigation-helper"))
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
        .arg(fixture("nextjs-coverage", "scanner-edge-cases"))
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
fn skipped_tests_do_not_cover_by_default_but_conditional_tests_do() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-coverage", "skipped-tests"))
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
        .arg(fixture("nextjs-coverage", "skipped-tests"))
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
        .arg(fixture("nextjs-coverage", "skipped-tests"))
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
        .arg(fixture("nextjs-coverage", "covered"))
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
        .arg(fixture("nextjs-coverage", "covered"))
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
        .arg(fixture("nextjs-coverage", "empty-app"))
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
        .arg(fixture("nextjs-routes", "base-url"))
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
        .arg(fixture("nextjs-routes", "external-url"))
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
        .arg(fixture("playwright-configs", "missing-test-dir"))
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
        .arg(fixture("playwright-configs", "invalid-project-config"))
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
        .arg(fixture("playwright-configs", "invalid-root-config"))
        .arg("check")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("expected string literal"));
}
