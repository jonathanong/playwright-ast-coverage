use crate::analysis::duplicates::build_duplicate_selectors;
use crate::analysis::types::{
    CoverageFetch, CoverageLinks, CoverageReport, CoverageRoute, CoverageSelector, Edge,
    SelectorCoverageKey, Summary, TestRef, UniqueSelectorPolicy,
};
use crate::config::Settings;
use crate::fsutil::relative_string;
use crate::routes::Route;
use crate::selectors;
use crate::url::is_ignored;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

type RouteCoverageEntry<'a> = BTreeMap<&'a str, (BTreeSet<String>, BTreeSet<String>, Vec<TestRef>)>;
type SelectorCoverageEntry = BTreeMap<SelectorCoverageKey, (CoverageLinks, Vec<TestRef>)>;
type FetchKey = (String, String);
type FetchCoverageEntry = BTreeMap<FetchKey, (BTreeSet<String>, Vec<TestRef>, BTreeSet<String>)>;

pub(crate) fn build_coverage(
    root: &Path,
    routes: &[Route],
    app_selectors: &[selectors::AppSelector],
    app_selector_occurrences: &[selectors::AppSelector],
    edges: &[Edge],
    settings: &Settings,
    unique_selector_policy: UniqueSelectorPolicy,
) -> CoverageReport {
    let ignored: Vec<String> = settings.ignore_routes.clone();
    let mut by_route: RouteCoverageEntry<'_> = BTreeMap::new();
    let mut by_selector: SelectorCoverageEntry = BTreeMap::new();
    let mut by_fetch: FetchCoverageEntry = BTreeMap::new();

    for edge in edges {
        match edge {
            Edge::Route {
                test_file,
                test_name,
                describe_path,
                route,
                url,
                ..
            } => {
                let entry = by_route
                    .entry(route.as_str())
                    .or_insert_with(|| (Default::default(), Default::default(), Vec::new()));
                entry.0.insert(test_file.clone());
                entry.1.insert(url.clone());
                entry.2.push(TestRef {
                    file: test_file.clone(),
                    name: test_name.clone(),
                    describe_path: describe_path.clone(),
                });
            }
            Edge::Selector {
                test_file,
                test_name,
                describe_path,
                app_file,
                attribute,
                value,
                selector,
            } => {
                let entry = by_selector
                    .entry((app_file.clone(), attribute.clone(), value.clone()))
                    .or_insert_with(|| ((Default::default(), Default::default()), Vec::new()));
                entry.0 .0.insert(test_file.clone());
                entry.0 .1.insert(selector.clone());
                entry.1.push(TestRef {
                    file: test_file.clone(),
                    name: test_name.clone(),
                    describe_path: describe_path.clone(),
                });
            }
            Edge::Fetch {
                test_file,
                test_name,
                describe_path,
                route_file,
                method,
                path,
                ..
            } => {
                let key = (method.clone(), path.clone());
                let entry = by_fetch
                    .entry(key)
                    .or_insert_with(|| (Default::default(), Vec::new(), Default::default()));
                entry.0.insert(test_file.clone());
                entry.1.push(TestRef {
                    file: test_file.clone(),
                    name: test_name.clone(),
                    describe_path: describe_path.clone(),
                });
                entry.2.insert(route_file.clone());
            }
        }
    }

    let mut coverage_routes = Vec::new();
    for route in routes {
        let (tests, urls, tests_detail) = by_route
            .get(route.pattern.as_str())
            .cloned()
            .unwrap_or_default();
        let covered = !tests.is_empty() || is_ignored(&route.pattern, &ignored);
        coverage_routes.push(CoverageRoute {
            route: route.pattern.clone(),
            file: relative_string(root, &route.file),
            covered,
            tests: tests.into_iter().collect(),
            tests_detail,
            urls: urls.into_iter().collect(),
        });
    }

    coverage_routes.sort_by(|a, b| a.route.cmp(&b.route).then_with(|| a.file.cmp(&b.file)));
    let mut coverage_selectors = Vec::new();
    for app_selector in app_selectors {
        let app_file = relative_string(root, &app_selector.file);
        let value = app_selector.display_value();
        let ((tests, selectors), tests_detail) = by_selector
            .get(&(
                app_file.clone(),
                app_selector.attribute.clone(),
                value.clone(),
            ))
            .cloned()
            .unwrap_or_default();
        let covered = !tests.is_empty();
        coverage_selectors.push(CoverageSelector {
            attribute: app_selector.attribute.clone(),
            value,
            file: app_file,
            covered,
            unsupported_dynamic: app_selector.unsupported_dynamic(),
            tests: tests.into_iter().collect(),
            tests_detail,
            selectors: selectors.into_iter().collect(),
        });
    }
    coverage_selectors.sort_by(|a, b| {
        a.attribute
            .cmp(&b.attribute)
            .then_with(|| a.value.cmp(&b.value))
            .then_with(|| a.file.cmp(&b.file))
    });

    let mut fetch_apis: Vec<CoverageFetch> = by_fetch
        .into_iter()
        .map(
            |((method, path), (tests, tests_detail, route_files))| CoverageFetch {
                covered: !tests.is_empty(),
                tests: tests.into_iter().collect(),
                tests_detail,
                route_files: route_files.into_iter().collect(),
                method,
                path,
            },
        )
        .collect();
    fetch_apis.sort_by(|a, b| a.method.cmp(&b.method).then_with(|| a.path.cmp(&b.path)));

    let total_routes = coverage_routes.len();
    let covered_routes = coverage_routes.iter().filter(|route| route.covered).count();
    let uncovered_routes = total_routes.saturating_sub(covered_routes);
    let total_selectors = coverage_selectors.len();
    let covered_selectors = coverage_selectors
        .iter()
        .filter(|selector| selector.covered)
        .count();
    let uncovered_selectors = total_selectors.saturating_sub(covered_selectors);
    let duplicate_selectors =
        build_duplicate_selectors(root, app_selector_occurrences, unique_selector_policy);
    let duplicate_selector_count = duplicate_selectors.len();
    let total_fetch_apis = fetch_apis.len();
    let covered_fetch_apis = fetch_apis.iter().filter(|f| f.covered).count();
    let uncovered_fetch_apis = total_fetch_apis.saturating_sub(covered_fetch_apis);

    CoverageReport {
        summary: Summary {
            total_routes,
            covered_routes,
            uncovered_routes,
            total_selectors,
            covered_selectors,
            uncovered_selectors,
            duplicate_selectors: duplicate_selector_count,
            total_fetch_apis,
            covered_fetch_apis,
            uncovered_fetch_apis,
        },
        routes: coverage_routes,
        selectors: coverage_selectors,
        duplicate_selectors,
        fetch_apis,
    }
}

pub(crate) fn has_configured_html_id_selector(settings: &Settings) -> bool {
    settings
        .selector_attributes
        .iter()
        .any(|attribute| attribute == selectors::HTML_ID_ATTRIBUTE)
        || settings
            .component_selector_attributes
            .values()
            .any(|attribute| attribute == selectors::HTML_ID_ATTRIBUTE)
}
