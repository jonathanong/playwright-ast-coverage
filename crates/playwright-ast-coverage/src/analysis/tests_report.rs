use crate::analysis::types::{Edge, TestEntry, TestsReport};
use crate::fsutil::relative_string;
use crate::selectors;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

type TestKey = (Arc<String>, Option<Arc<String>>, Arc<Vec<String>>);
type TestBuckets = (
    BTreeSet<String>,
    BTreeSet<String>,
    BTreeSet<String>,
    BTreeSet<String>,
);

pub(crate) fn build_tests_report(edges: &[Edge], files: &[PathBuf], root: &Path) -> TestsReport {
    let filter_files: BTreeSet<String> = files.iter().map(|f| input_file(root, f)).collect();

    let mut by_test: BTreeMap<TestKey, TestBuckets> = BTreeMap::new();

    for edge in edges {
        match edge {
            Edge::Route {
                test_file,
                test_name,
                describe_path,
                route,
                ..
            } => {
                if !filter_files.is_empty() && !filter_files.contains(test_file.as_str()) {
                    continue;
                }
                let key: TestKey = (test_file.clone(), test_name.clone(), describe_path.clone());
                by_test.entry(key).or_default().2.insert(route.to_string());
            }
            Edge::Selector {
                test_file,
                test_name,
                describe_path,
                attribute,
                value,
                ..
            } => {
                if !filter_files.is_empty() && !filter_files.contains(test_file.as_str()) {
                    continue;
                }
                let key: TestKey = (test_file.clone(), test_name.clone(), describe_path.clone());
                let entry = by_test.entry(key).or_default();
                if attribute == selectors::HTML_ID_ATTRIBUTE {
                    entry.1.insert(value.clone());
                } else {
                    entry.0.insert(value.clone());
                }
            }
            Edge::Fetch {
                test_file,
                test_name,
                describe_path,
                method,
                path,
                ..
            } => {
                if !filter_files.is_empty() && !filter_files.contains(test_file.as_str()) {
                    continue;
                }
                let key: TestKey = (test_file.clone(), test_name.clone(), describe_path.clone());
                by_test
                    .entry(key)
                    .or_default()
                    .3
                    .insert(format!("{method} {path}"));
            }
        }
    }

    let tests = by_test
        .into_iter()
        .map(
            |((file, name, describe_path), (test_ids, html_ids, routes, fetch_apis))| TestEntry {
                file: Arc::unwrap_or_clone(file),
                name: name.map(Arc::unwrap_or_clone),
                describe_path: Arc::unwrap_or_clone(describe_path),
                test_ids: test_ids.into_iter().collect(),
                html_ids: html_ids.into_iter().collect(),
                routes: routes.into_iter().collect(),
                fetch_apis: fetch_apis.into_iter().collect(),
            },
        )
        .collect();

    TestsReport { tests }
}

pub(crate) fn print_tests_text(report: &TestsReport) {
    for entry in &report.tests {
        if let Some(name) = &entry.name {
            let path_prefix = if entry.describe_path.is_empty() {
                String::new()
            } else {
                format!("{} > ", entry.describe_path.join(" > "))
            };
            println!("{} > {}{}", entry.file, path_prefix, name);
        } else {
            println!("{}", entry.file);
        }
        for route in &entry.routes {
            println!("  route: {route}");
        }
        for fetch_api in &entry.fetch_apis {
            println!("  fetch: {fetch_api}");
        }
        for test_id in &entry.test_ids {
            println!("  test-id: {test_id}");
        }
        for html_id in &entry.html_ids {
            println!("  html-id: {html_id}");
        }
    }
}

fn input_file(root: &Path, file: &Path) -> String {
    if file.is_absolute() {
        return relative_string(root, file);
    }
    relative_string(root, &root.join(file))
}
