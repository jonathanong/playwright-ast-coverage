use crate::analysis::output::{build_related_report, print_coverage_text, print_edges_text};
use crate::analysis::tests_report::print_tests_text;
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
                test_file: "tests/e2e/app.spec.ts".to_string(),
                test_name: None,
                describe_path: vec![],
                route_file: "web/app/page.tsx".to_string(),
                route: "/".to_string(),
                url: "/".to_string(),
            },
            Edge::Selector {
                test_file: "tests/e2e/app.spec.ts".to_string(),
                test_name: None,
                describe_path: vec![],
                app_file: "web/app/page.tsx".to_string(),
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
            test_file: "tests/e2e/app.spec.ts".to_string(),
            test_name: Some("visits home".to_string()),
            describe_path: vec!["Suite".to_string()],
            route_file: "web/app/page.tsx".to_string(),
            route: "/".to_string(),
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
            test_file: "tests/e2e/app.spec.ts".to_string(),
            test_name: None,
            describe_path: vec![],
            route_file: "web/app/page.tsx".to_string(),
            route: "/".to_string(),
            url: "/".to_string(),
        },
        Edge::Fetch {
            test_file: "tests/e2e/app.spec.ts".to_string(),
            test_name: None,
            describe_path: vec![],
            route_file: "web/app/page.tsx".to_string(),
            route: "/".to_string(),
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
