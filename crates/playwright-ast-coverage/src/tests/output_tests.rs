use crate::analysis::output::{build_related_report, print_coverage_text, print_edges_text};
use crate::analysis::tests_report::{build_tests_report, print_tests_text};
use crate::analysis::types::{
    CoverageFetch, CoverageReport, CoverageRoute, CoverageSelector, DuplicateSelector, Edge,
    EdgeReport, Summary, TestEntry, TestsReport,
};
use std::path::PathBuf;

#[test]
fn text_printers_cover_routes_and_selectors() {
    let coverage = CoverageReport {
        summary: Summary {
            total_routes: 1,
            covered_routes: 0,
            uncovered_routes: 1,
            total_selectors: 1,
            covered_selectors: 0,
            uncovered_selectors: 1,
            duplicate_selectors: 1,
            total_fetch_apis: 0,
            covered_fetch_apis: 0,
            uncovered_fetch_apis: 0,
        },
        routes: vec![CoverageRoute {
            route: "/missing".to_string(),
            file: "web/app/missing/page.tsx".to_string(),
            covered: false,
            tests: vec![],
            tests_detail: vec![],
            urls: vec![],
        }],
        selectors: vec![CoverageSelector {
            attribute: "data-testid".to_string(),
            value: "missing".to_string(),
            file: "web/app/page.tsx".to_string(),
            covered: false,
            unsupported_dynamic: false,
            tests: vec![],
            tests_detail: vec![],
            selectors: vec![],
        }],
        duplicate_selectors: vec![DuplicateSelector {
            attribute: "data-testid".to_string(),
            value: "missing".to_string(),
            file: "web/app/other.tsx".to_string(),
        }],
        fetch_apis: vec![],
    };
    print_coverage_text(&coverage);

    let edges = EdgeReport {
        edges: vec![
            Edge::Route {
                test_file: std::sync::Arc::new("tests/e2e/app.spec.ts".to_string()),
                test_name: None,
                describe_path: std::sync::Arc::new(vec![]),
                route_file: std::sync::Arc::new("web/app/page.tsx".to_string()),
                route: std::sync::Arc::new("/".to_string()),
                url: std::sync::Arc::new("/".to_string()),
            },
            Edge::Selector {
                test_file: std::sync::Arc::new("tests/e2e/app.spec.ts".to_string()),
                test_name: None,
                describe_path: std::sync::Arc::new(vec![]),
                app_file: std::sync::Arc::new("web/app/page.tsx".to_string()),
                attribute: "data-testid".to_string(),
                value: "save".to_string(),
                selector: "getByTestId(save)".to_string(),
            },
        ],
    };
    print_edges_text(&edges);
}

#[test]
fn text_printer_covers_fetch_edges() {
    let edges = EdgeReport {
        edges: vec![Edge::Fetch {
            test_file: std::sync::Arc::new("tests/e2e/app.spec.ts".to_string()),
            test_name: Some(std::sync::Arc::new("visits home".to_string())),
            describe_path: std::sync::Arc::new(vec!["Suite".to_string()]),
            route_file: std::sync::Arc::new("web/app/page.tsx".to_string()),
            route: std::sync::Arc::new("/".to_string()),
            method: "GET".to_string(),
            path: "/api/health".to_string(),
            side: "server".to_string(),
            cached: false,
        }],
    };
    print_edges_text(&edges);
}

#[test]
fn coverage_text_covers_fetch_apis() {
    let coverage = CoverageReport {
        summary: Summary {
            total_routes: 0,
            covered_routes: 0,
            uncovered_routes: 0,
            total_selectors: 0,
            covered_selectors: 0,
            uncovered_selectors: 0,
            duplicate_selectors: 0,
            total_fetch_apis: 1,
            covered_fetch_apis: 0,
            uncovered_fetch_apis: 1,
        },
        routes: vec![],
        selectors: vec![],
        duplicate_selectors: vec![],
        fetch_apis: vec![CoverageFetch {
            method: "GET".to_string(),
            path: "/api/missing".to_string(),
            covered: false,
            tests: vec![],
            tests_detail: vec![],
            route_files: vec!["web/app/page.tsx".to_string()],
        }],
    };
    print_coverage_text(&coverage);
}

#[test]
fn related_report_includes_fetch_apis() {
    let root = std::path::Path::new("/repo");
    let edges = vec![
        Edge::Route {
            test_file: std::sync::Arc::new("tests/e2e/app.spec.ts".to_string()),
            test_name: None,
            describe_path: std::sync::Arc::new(vec![]),
            route_file: std::sync::Arc::new("web/app/page.tsx".to_string()),
            route: std::sync::Arc::new("/".to_string()),
            url: std::sync::Arc::new("/".to_string()),
        },
        Edge::Fetch {
            test_file: std::sync::Arc::new("tests/e2e/app.spec.ts".to_string()),
            test_name: None,
            describe_path: std::sync::Arc::new(vec![]),
            route_file: std::sync::Arc::new("web/app/page.tsx".to_string()),
            route: std::sync::Arc::new("/".to_string()),
            method: "GET".to_string(),
            path: "/api/health".to_string(),
            side: "server".to_string(),
            cached: false,
        },
    ];
    let related = build_related_report(root, &edges, &[PathBuf::from("/repo/web/app/page.tsx")]);
    assert!(related.tests.contains(&"tests/e2e/app.spec.ts".to_string()));
    assert!(related.fetch_apis.contains(&"GET /api/health".to_string()));
}

#[test]
fn print_tests_text_covers_html_ids() {
    let report = TestsReport {
        tests: vec![TestEntry {
            file: "tests/e2e/app.spec.ts".to_string(),
            name: Some("visits home".to_string()),
            describe_path: vec![],
            test_ids: vec![],
            html_ids: vec!["main-nav".to_string()],
            routes: vec![],
            fetch_apis: vec![],
        }],
    };
    print_tests_text(&report);
}

#[test]
fn print_tests_text_with_describe_path_and_unnamed_entry() {
    let report = TestsReport {
        tests: vec![
            TestEntry {
                file: "tests/e2e/app.spec.ts".to_string(),
                name: Some("my test".to_string()),
                describe_path: vec!["Suite".to_string(), "Nested".to_string()],
                test_ids: vec![],
                html_ids: vec![],
                routes: vec!["/".to_string()],
                fetch_apis: vec!["GET /api/data".to_string()],
            },
            TestEntry {
                file: "tests/e2e/app.spec.ts".to_string(),
                name: None,
                describe_path: vec![],
                test_ids: vec![],
                html_ids: vec![],
                routes: vec![],
                fetch_apis: vec![],
            },
        ],
    };
    print_tests_text(&report);
}

#[test]
fn edge_report_json_schema_is_stable_with_arc_fields() {
    let report = EdgeReport {
        edges: vec![
            Edge::Route {
                test_file: std::sync::Arc::new("tests/e2e/app.spec.ts".to_string()),
                test_name: None,
                describe_path: std::sync::Arc::new(vec![]),
                route_file: std::sync::Arc::new("web/app/page.tsx".to_string()),
                route: std::sync::Arc::new("/".to_string()),
                url: std::sync::Arc::new("/api/health".to_string()),
            },
            Edge::Selector {
                test_file: std::sync::Arc::new("tests/e2e/app.spec.ts".to_string()),
                test_name: Some(std::sync::Arc::new("visits home".to_string())),
                describe_path: std::sync::Arc::new(vec!["Suite".to_string()]),
                app_file: std::sync::Arc::new("web/app/page.tsx".to_string()),
                attribute: "data-testid".to_string(),
                value: "save".to_string(),
                selector: "getByTestId(save)".to_string(),
            },
            Edge::Fetch {
                test_file: std::sync::Arc::new("tests/e2e/app.spec.ts".to_string()),
                test_name: Some(std::sync::Arc::new("loads home".to_string())),
                describe_path: std::sync::Arc::new(vec![]),
                route_file: std::sync::Arc::new("web/app/page.tsx".to_string()),
                route: std::sync::Arc::new("/".to_string()),
                method: "GET".to_string(),
                path: "/api/health".to_string(),
                side: "server".to_string(),
                cached: false,
            },
        ],
    };

    let value = serde_json::to_value(report).unwrap();
    let edges = value["edges"].as_array().unwrap();

    let route = &edges[0];
    assert_eq!(route["kind"], "route");
    assert_eq!(route["testFile"], "tests/e2e/app.spec.ts");
    assert_eq!(route["routeFile"], "web/app/page.tsx");
    assert_eq!(route["route"], "/");
    assert_eq!(route["url"], "/api/health");
    assert!(!route.as_object().unwrap().contains_key("testName"));
    assert!(!route.as_object().unwrap().contains_key("describePath"));

    let selector = &edges[1];
    assert_eq!(selector["kind"], "selector");
    assert_eq!(selector["testFile"], "tests/e2e/app.spec.ts");
    assert_eq!(selector["testName"], "visits home");
    assert_eq!(selector["describePath"], serde_json::json!(["Suite"]));
    assert_eq!(selector["appFile"], "web/app/page.tsx");

    let fetch = &edges[2];
    assert_eq!(fetch["kind"], "fetch");
    assert_eq!(fetch["testFile"], "tests/e2e/app.spec.ts");
    assert_eq!(fetch["testName"], "loads home");
    assert!(!fetch.as_object().unwrap().contains_key("describePath"));
    assert_eq!(fetch["routeFile"], "web/app/page.tsx");
    assert_eq!(fetch["route"], "/");
    assert_eq!(fetch["method"], "GET");
    assert_eq!(fetch["path"], "/api/health");
    assert_eq!(fetch["side"], "server");
    assert!(fetch["cached"].as_bool().is_some_and(|cached| !cached));
}

#[test]
fn build_tests_report_produces_entries_with_routes_and_fetch_apis() {
    let root = std::path::Path::new("/repo");
    let edges = vec![
        Edge::Route {
            test_file: std::sync::Arc::new("tests/e2e/app.spec.ts".to_string()),
            test_name: Some(std::sync::Arc::new("visits home".to_string())),
            describe_path: std::sync::Arc::new(vec!["Suite".to_string()]),
            route_file: std::sync::Arc::new("web/app/page.tsx".to_string()),
            route: std::sync::Arc::new("/".to_string()),
            url: std::sync::Arc::new("/".to_string()),
        },
        Edge::Fetch {
            test_file: std::sync::Arc::new("tests/e2e/app.spec.ts".to_string()),
            test_name: Some(std::sync::Arc::new("visits home".to_string())),
            describe_path: std::sync::Arc::new(vec!["Suite".to_string()]),
            route_file: std::sync::Arc::new("web/app/page.tsx".to_string()),
            route: std::sync::Arc::new("/".to_string()),
            method: "GET".to_string(),
            path: "/api/health".to_string(),
            side: "server".to_string(),
            cached: false,
        },
    ];
    let report = build_tests_report(&edges, &[], root);
    assert_eq!(report.tests.len(), 1);
    assert_eq!(report.tests[0].name.as_deref(), Some("visits home"));
    assert_eq!(report.tests[0].describe_path, vec!["Suite".to_string()]);
    assert!(report.tests[0].routes.contains(&"/".to_string()));
    assert!(report.tests[0]
        .fetch_apis
        .contains(&"GET /api/health".to_string()));
}

#[test]
fn build_tests_report_groups_selector_edges_by_attribute() {
    let root = std::path::Path::new("/repo");
    let edges = vec![
        Edge::Selector {
            test_file: std::sync::Arc::new("tests/e2e/app.spec.ts".to_string()),
            test_name: Some(std::sync::Arc::new("visits home".to_string())),
            describe_path: std::sync::Arc::new(vec![]),
            app_file: std::sync::Arc::new("web/app/page.tsx".to_string()),
            attribute: "id".to_string(),
            value: "main-nav".to_string(),
            selector: "#main-nav".to_string(),
        },
        Edge::Selector {
            test_file: std::sync::Arc::new("tests/e2e/app.spec.ts".to_string()),
            test_name: Some(std::sync::Arc::new("visits home".to_string())),
            describe_path: std::sync::Arc::new(vec![]),
            app_file: std::sync::Arc::new("web/app/page.tsx".to_string()),
            attribute: "data-testid".to_string(),
            value: "save".to_string(),
            selector: "getByTestId(save)".to_string(),
        },
    ];
    let report = build_tests_report(&edges, &[], root);
    assert_eq!(report.tests.len(), 1);
    assert!(report.tests[0].html_ids.contains(&"main-nav".to_string()));
    assert!(report.tests[0].test_ids.contains(&"save".to_string()));
}

#[test]
fn build_tests_report_with_absolute_file_path_filter() {
    let root = std::path::Path::new("/repo");
    let edges = vec![Edge::Route {
        test_file: std::sync::Arc::new("tests/e2e/app.spec.ts".to_string()),
        test_name: Some(std::sync::Arc::new("visits home".to_string())),
        describe_path: std::sync::Arc::new(vec![]),
        route_file: std::sync::Arc::new("web/app/page.tsx".to_string()),
        route: std::sync::Arc::new("/".to_string()),
        url: std::sync::Arc::new("/".to_string()),
    }];
    // Pass an absolute path as the file filter — exercises the absolute branch in input_file()
    let abs_filter = std::path::PathBuf::from("/repo/tests/e2e/app.spec.ts");
    let report = build_tests_report(&edges, &[abs_filter], root);
    assert_eq!(report.tests.len(), 1);
    assert_eq!(report.tests[0].name.as_deref(), Some("visits home"));
}
