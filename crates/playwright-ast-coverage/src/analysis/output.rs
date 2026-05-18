use crate::analysis::types::{CoverageReport, Edge, EdgeReport, RelatedReport};
use crate::fsutil::relative_string;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(crate) fn print_coverage_text(report: &CoverageReport) {
    println!("Routes: {}", report.summary.total_routes);
    println!("Covered routes: {}", report.summary.covered_routes);
    println!("Uncovered routes: {}", report.summary.uncovered_routes);
    println!("Selectors: {}", report.summary.total_selectors);
    println!("Covered selectors: {}", report.summary.covered_selectors);
    println!(
        "Uncovered selectors: {}",
        report.summary.uncovered_selectors
    );
    println!(
        "Duplicate selectors: {}",
        report.summary.duplicate_selectors
    );

    if report.summary.uncovered_routes == 0
        && report.summary.uncovered_selectors == 0
        && report.summary.duplicate_selectors == 0
    {
        println!();
        println!("All routes and selectors covered.");
        return;
    }

    if report.summary.uncovered_routes > 0 {
        println!();
        println!("Uncovered routes:");
        for route in report.routes.iter().filter(|route| !route.covered) {
            println!("  {}  {}", route.route, route.file);
        }
    }

    if report.summary.uncovered_selectors > 0 {
        println!();
        println!("Uncovered selectors:");
        for selector in report.selectors.iter().filter(|selector| !selector.covered) {
            println!(
                "  [{}=\"{}\"]  {}",
                selector.attribute, selector.value, selector.file
            );
        }
    }

    if report.summary.duplicate_selectors > 0 {
        println!();
        println!("Duplicate selectors:");
        for selector in &report.duplicate_selectors {
            println!(
                "  [{}=\"{}\"]  {}",
                selector.attribute, selector.value, selector.file
            );
        }
    }
}

pub(crate) fn print_edges_text(report: &EdgeReport) {
    for edge in &report.edges {
        match edge {
            Edge::Route {
                test_file,
                route_file,
                route,
                url,
                ..
            } => println!("{test_file} -> {route_file} ({route}, {url})"),
            Edge::Selector {
                test_file,
                app_file,
                attribute,
                value,
                selector,
                ..
            } => println!("{test_file} -> {app_file} ([{attribute}=\"{value}\"], {selector})"),
            Edge::Fetch {
                test_file,
                route,
                method,
                path,
                ..
            } => println!("{test_file} -> {method} {path} (via {route})"),
        }
    }
}

pub(crate) fn build_related_report(
    root: &Path,
    edges: &[Edge],
    files: &[PathBuf],
) -> RelatedReport {
    let related_files: BTreeSet<String> = files
        .iter()
        .map(|file| related_input_file(root, file))
        .collect();
    let mut tests = BTreeSet::new();
    let mut fetch_apis = BTreeSet::new();

    for edge in edges {
        match edge {
            Edge::Route {
                test_file,
                route_file,
                ..
            } if related_files.contains(route_file.as_ref()) => {
                tests.insert(test_file.to_string());
            }
            Edge::Selector {
                test_file,
                app_file,
                ..
            } if related_files.contains(app_file.as_ref()) => {
                tests.insert(test_file.to_string());
            }
            Edge::Fetch {
                test_file,
                route_file,
                method,
                path,
                ..
            } if related_files.contains(route_file.as_ref()) => {
                tests.insert(test_file.to_string());
                fetch_apis.insert(format!("{method} {path}"));
            }
            _ => {}
        }
    }

    RelatedReport {
        tests: tests.into_iter().collect(),
        fetch_apis: fetch_apis.into_iter().collect(),
    }
}

pub(crate) fn print_related_text(report: &RelatedReport) {
    for test in &report.tests {
        println!("{test}");
    }
}

fn related_input_file(root: &Path, file: &Path) -> String {
    if file.is_absolute() {
        return relative_string(root, file);
    }
    relative_string(root, &root.join(file))
}
