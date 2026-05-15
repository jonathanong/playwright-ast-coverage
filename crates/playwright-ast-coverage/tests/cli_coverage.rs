mod common;

use assert_cmd::Command;
use common::{fixture, has_route_edge};
use predicates::prelude::*;
use serde_json::Value;

#[test]
fn coverage_json_reports_uncovered_routes() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-coverage", "uncovered"))
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
        .arg(fixture("nextjs-coverage", "ignored"))
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
        .arg(fixture("nextjs-coverage", "covered"))
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
        .arg(fixture("nextjs-coverage", "covered"))
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
        .current_dir(fixture("nextjs-coverage", "covered"))
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
        .arg(fixture("nextjs-coverage", "sort-tiebreakers"))
        .arg("--json")
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""uncoveredRoutes": 0"#))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 0"#));
}

#[test]
fn check_can_fail_on_duplicate_test_id_literals() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-coverage", "sort-tiebreakers"))
        .arg("--assert-unique-test-ids")
        .arg("check")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Duplicate selectors:"))
        .stdout(predicate::str::contains(r#"[data-testid="dup"]"#));
}

#[test]
fn deprecated_unique_selectors_flag_still_checks_selectors() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-coverage", "sort-tiebreakers"))
        .arg("--assert-unique-selectors")
        .arg("check")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Duplicate selectors:"))
        .stdout(predicate::str::contains(r#"[data-testid="dup"]"#));
}

#[test]
fn check_can_fail_on_duplicate_html_id_literals_without_html_id_coverage() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-html-ids", "unique-html-ids"))
        .arg("--assert-unique-html-ids")
        .arg("check")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\nUncovered selectors:\n").not())
        .stdout(predicate::str::contains("Duplicate selectors:"))
        .stdout(predicate::str::contains(r#"[id="save"]"#));
}

#[test]
fn test_ids_and_html_ids_with_the_same_value_are_not_duplicates() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-selectors", "unique-cross-attribute"))
        .arg("--assert-unique-test-ids")
        .arg("--assert-unique-html-ids")
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("Duplicate selectors: 0"));
}

#[test]
fn coverage_text_reports_uncovered_routes() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-coverage", "uncovered"))
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
        .arg(fixture("nextjs-coverage", "codebase-intel"))
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
        .arg(fixture("nextjs-coverage", "codebase-intel"))
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
        .arg(fixture("nextjs-coverage", "codebase-intel"))
        .arg("edges")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "tests/e2e/users.spec.ts -> packages/web/app/users/[id]/page.tsx",
        ));
}

#[test]
fn duplicate_slash_empty_segments_do_not_cover_dynamic_routes() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-routes", "empty-segment-route"))
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
fn route_specificity_keeps_static_routes_from_covering_dynamic_siblings() {
    let output = Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-routes", "realistic-route-selector-edge-cases"))
        .arg("--allow-skipped-tests")
        .arg("edges")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let edges = report["edges"].as_array().unwrap();

    assert!(has_route_edge(edges, "/admin/crm", "/admin/crm"));
    assert!(has_route_edge(edges, "/chat/support", "/chat/support"));
    assert!(has_route_edge(
        edges,
        "/chat/support/new",
        "/chat/support/new"
    ));
    assert!(has_route_edge(
        edges,
        "/chat/support/:threadId",
        "/chat/support/x"
    ));
    assert!(has_route_edge(
        edges,
        "/communities/create",
        "/communities/create"
    ));
    assert!(has_route_edge(edges, "/:topicType/:id", "/discussion/abc"));
    assert!(has_route_edge(
        edges,
        "/admin/crm/:contactId",
        "/admin/crm/x"
    ));
    assert!(has_route_edge(edges, "/discussions", "/discussions"));

    assert!(!has_route_edge(edges, "/:topicType/:id", "/admin/crm"));
    assert!(!has_route_edge(
        edges,
        "/chat/:conversationId",
        "/chat/support"
    ));
    assert!(!has_route_edge(
        edges,
        "/chat/support/:threadId",
        "/chat/support/new"
    ));
    assert!(!has_route_edge(
        edges,
        "/communities/:slug",
        "/communities/create"
    ));
}

#[test]
fn skipped_navigation_helper_catalog_requires_allow_skipped_tests() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-routes", "realistic-route-selector-edge-cases"))
        .arg("--json")
        .arg("check")
        .assert()
        .code(1)
        .stdout(predicate::str::contains(r#""route": "/catalog-only""#))
        .stdout(predicate::str::contains(
            "\"route\": \"/catalog-only\",\n      \"file\": \"app/catalog-only/page.tsx\",\n      \"covered\": false",
        ));

    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-routes", "realistic-route-selector-edge-cases"))
        .arg("--allow-skipped-tests")
        .arg("--json")
        .arg("check")
        .assert()
        .code(1)
        .stdout(predicate::str::contains(r#""uncoveredRoutes": 0"#))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 1"#))
        .stdout(predicate::str::contains(r#""value": "rss-feed-link""#))
        .stdout(predicate::str::contains(r#""covered": true"#))
        .stdout(predicate::str::contains(r#""value": "{dataPw}""#))
        .stdout(predicate::str::contains(r#""unsupportedDynamic": true"#));
}

#[test]
fn selector_roots_and_excludes_are_configurable() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-selectors", "selector-roots"))
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
fn selector_coverage_reports_all_selectors_covered() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-selectors", "selector-covered"))
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
        .arg(fixture("nextjs-selectors", "selector-uncovered"))
        .arg("check")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Uncovered selectors:"))
        .stdout(predicate::str::contains(r#"[data-testid="save"]"#));
}

#[test]
fn configured_id_selector_coverage_does_not_require_html_ids() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-selectors", "selector-id-configured"))
        .arg("check")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Uncovered selectors:"))
        .stdout(predicate::str::contains(r#"[id="save"]"#));
}

#[test]
fn selector_coverage_supports_fuzzy_templates() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-selectors", "selector-fuzzy"))
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
        .arg(fixture("nextjs-selectors", "selector-custom"))
        .arg("--json")
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""attribute": "data-test""#))
        .stdout(predicate::str::contains(r#""attribute": "data-test-id""#))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 0"#));
}

#[test]
fn selector_coverage_supports_component_attribute_mapping() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-selectors", "component-selector-attributes"))
        .arg("--json")
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""attribute": "data-pw""#))
        .stdout(predicate::str::contains(r#""value": "save""#))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 0"#));
}

#[test]
fn selector_coverage_supports_optional_html_ids() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-html-ids", "html-ids-covered"))
        .arg("--json")
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""attribute": "id""#))
        .stdout(predicate::str::contains(r##""#save""##))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 0"#));
}

#[test]
fn selector_coverage_reports_uncovered_html_ids_when_enabled() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-html-ids", "html-ids-uncovered"))
        .arg("check")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Uncovered selectors:"))
        .stdout(predicate::str::contains(r#"[id="publish"]"#));
}

#[test]
fn html_ids_are_ignored_by_default() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-html-ids", "html-ids-default"))
        .arg("--json")
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""totalSelectors": 0"#));
}

#[test]
fn selector_coverage_can_be_disabled() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-selectors", "selector-disabled"))
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
        .arg(fixture("nextjs-selectors", "selector-unsupported"))
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
        .arg(fixture("nextjs-test-ids", "testid-attribute"))
        .arg("--json")
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""attribute": "data-pw""#))
        .stdout(predicate::str::contains(r#""uncoveredSelectors": 0"#));
}

#[test]
fn edges_json_outputs_selector_edges() {
    Command::cargo_bin("playwright-ast-coverage")
        .unwrap()
        .arg("--root")
        .arg(fixture("nextjs-selectors", "selector-covered"))
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
