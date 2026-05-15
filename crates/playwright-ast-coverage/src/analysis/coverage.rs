use crate::analysis::duplicates::build_duplicate_selectors;
use crate::analysis::types::{
    CoverageLinks, CoverageReport, CoverageRoute, CoverageSelector, Edge, SelectorCoverageKey,
    Summary, UniqueSelectorPolicy,
};
use crate::config::Settings;
use crate::fsutil::relative_string;
use crate::routes::Route;
use crate::selectors;
use crate::url::is_ignored;
use std::collections::BTreeMap;
use std::path::Path;

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
    let mut by_route: BTreeMap<
        &str,
        (
            std::collections::BTreeSet<String>,
            std::collections::BTreeSet<String>,
        ),
    > = BTreeMap::new();
    let mut by_selector: BTreeMap<SelectorCoverageKey, CoverageLinks> = BTreeMap::new();

    for edge in edges {
        match edge {
            Edge::Route {
                test_file,
                route,
                url,
                ..
            } => {
                let entry = by_route
                    .entry(route.as_str())
                    .or_insert_with(|| (Default::default(), Default::default()));
                entry.0.insert(test_file.clone());
                entry.1.insert(url.clone());
            }
            Edge::Selector {
                test_file,
                app_file,
                attribute,
                value,
                selector,
            } => {
                let entry = by_selector
                    .entry((app_file.clone(), attribute.clone(), value.clone()))
                    .or_insert_with(|| (Default::default(), Default::default()));
                entry.0.insert(test_file.clone());
                entry.1.insert(selector.clone());
            }
        }
    }

    let mut coverage_routes = Vec::new();
    for route in routes {
        let (tests, urls) = by_route
            .get(route.pattern.as_str())
            .cloned()
            .unwrap_or_default();
        let covered = !tests.is_empty() || is_ignored(&route.pattern, &ignored);
        coverage_routes.push(CoverageRoute {
            route: route.pattern.clone(),
            file: relative_string(root, &route.file),
            covered,
            tests: tests.into_iter().collect(),
            urls: urls.into_iter().collect(),
        });
    }

    coverage_routes.sort_by(|a, b| a.route.cmp(&b.route).then_with(|| a.file.cmp(&b.file)));
    let mut coverage_selectors = Vec::new();
    for app_selector in app_selectors {
        let app_file = relative_string(root, &app_selector.file);
        let value = app_selector.display_value();
        let (tests, selectors) = by_selector
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
            selectors: selectors.into_iter().collect(),
        });
    }
    coverage_selectors.sort_by(|a, b| {
        a.attribute
            .cmp(&b.attribute)
            .then_with(|| a.value.cmp(&b.value))
            .then_with(|| a.file.cmp(&b.file))
    });

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

    CoverageReport {
        summary: Summary {
            total_routes,
            covered_routes,
            uncovered_routes,
            total_selectors,
            covered_selectors,
            uncovered_selectors,
            duplicate_selectors: duplicate_selector_count,
        },
        routes: coverage_routes,
        selectors: coverage_selectors,
        duplicate_selectors,
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
