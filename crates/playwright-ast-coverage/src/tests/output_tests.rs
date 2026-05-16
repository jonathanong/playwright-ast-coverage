use crate::analysis::output::{print_coverage_text, print_edges_text};
use crate::analysis::types::{
    CoverageReport, CoverageRoute, CoverageSelector, DuplicateSelector, Edge, EdgeReport, Summary,
};

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
