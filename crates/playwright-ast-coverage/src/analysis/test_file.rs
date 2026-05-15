use crate::analysis::context::{DiscoveredTestFile, TestAnalysisContext};
use crate::analysis::routes_index::route_specificity;
use crate::analysis::types::Edge;
use crate::fsutil::relative_string;
use crate::matcher;
use crate::selectors;
use crate::url::normalize_url;
use crate::{ast, playwright_urls};
use anyhow::Result;

pub(crate) fn analyze_test_file(
    test_file: &DiscoveredTestFile,
    context: &TestAnalysisContext<'_>,
) -> Result<Vec<Edge>> {
    let source = std::fs::read_to_string(&test_file.path)?;
    let rel_test_file = relative_string(context.root, &test_file.path);
    let mut edges = Vec::new();
    let base_urls = test_file.base_urls();
    let test_id_attributes = test_file.test_id_attributes();

    let (raw_urls, playwright_selectors) =
        ast::with_program(&test_file.path, &source, |program, source| {
            let raw_urls = playwright_urls::extract_playwright_url_occurrences_from_program(
                program,
                source,
                context.navigation_helpers,
            );
            let playwright_selectors = if context.app_selector_targets.is_empty() {
                Vec::new()
            } else {
                selectors::extract_playwright_selector_occurrences_from_program(
                    program,
                    source,
                    context.selector_regexes,
                    &test_id_attributes,
                )
            };
            (raw_urls, playwright_selectors)
        })?;

    for raw_url in raw_urls {
        if !context.test_policy.allows(raw_url.status) {
            continue;
        }
        let Some(url) = normalize_url(&raw_url.value, &base_urls) else {
            continue;
        };
        let ref_segments = matcher::reference_segments(&url);
        let matching_routes: Vec<_> = context
            .route_index
            .candidates(&ref_segments)
            .into_iter()
            .filter(|route| matcher::matches_segments(&ref_segments, &route.segments))
            .collect();
        let Some(best_specificity) = matching_routes
            .iter()
            .map(|route| route_specificity(&route.segments))
            .max()
        else {
            continue;
        };
        for route in matching_routes
            .into_iter()
            .filter(|route| route_specificity(&route.segments) == best_specificity)
        {
            edges.push(Edge::Route {
                test_file: rel_test_file.clone(),
                route_file: route.route_file.clone(),
                route: route.pattern.clone(),
                url: url.clone(),
            });
        }
    }

    if !context.app_selector_targets.is_empty() {
        for playwright_selector in &playwright_selectors {
            if !context.test_policy.allows(playwright_selector.status) {
                continue;
            }
            for app_selector in context.selector_index.matches(&playwright_selector.value) {
                edges.push(Edge::Selector {
                    test_file: rel_test_file.clone(),
                    app_file: app_selector.app_file.clone(),
                    attribute: app_selector.selector.attribute.clone(),
                    value: app_selector.value.clone(),
                    selector: playwright_selector.value.selector.clone(),
                });
            }
        }
    }

    Ok(edges)
}
