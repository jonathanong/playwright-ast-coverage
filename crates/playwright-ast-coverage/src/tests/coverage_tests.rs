use crate::analysis::coverage::build_coverage;
use crate::analysis::types::{Edge, UniqueSelectorPolicy};
use crate::config::Settings;
use crate::routes::Route;
use crate::selectors::{self, AppSelectorValue};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn default_settings(selector_attributes: Vec<String>) -> Settings {
    Settings {
        frontend_root: "web/app".to_string(),
        playwright_configs: vec![],
        project: None,
        test_include: vec![],
        test_exclude: vec![],
        ignore_routes: vec![],
        navigation_helpers: vec![],
        selector_attributes,
        component_selector_attributes: BTreeMap::new(),
        html_ids: false,
        selector_roots: vec!["web/app".to_string()],
        selector_include: vec![],
        selector_exclude: vec![],
    }
}

#[test]
fn coverage_sort_uses_file_as_tiebreaker() {
    let root = Path::new("/repo");
    let routes = vec![
        Route {
            file: PathBuf::from("/repo/web/app/a/page.tsx"),
            pattern: "/same".to_string(),
        },
        Route {
            file: PathBuf::from("/repo/web/app/b/page.tsx"),
            pattern: "/same".to_string(),
        },
    ];
    let settings = default_settings(vec!["data-testid".to_string(), "data-pw".to_string()]);
    let report = build_coverage(
        root,
        &routes,
        &[],
        &[],
        &[],
        &settings,
        UniqueSelectorPolicy::default(),
    );
    assert_eq!(report.routes[0].file, "web/app/a/page.tsx");
    assert_eq!(report.routes[1].file, "web/app/b/page.tsx");
}

#[test]
fn selector_coverage_sorts_and_counts_uncovered() {
    let root = Path::new("/repo");
    let app_selectors = vec![selectors::AppSelector {
        file: PathBuf::from("/repo/web/app/page.tsx"),
        attribute: "data-testid".to_string(),
        value: AppSelectorValue::Exact("save".to_string()),
    }];
    let settings = default_settings(vec!["data-testid".to_string()]);
    let report = build_coverage(
        root,
        &[],
        &app_selectors,
        &app_selectors,
        &[],
        &settings,
        UniqueSelectorPolicy::default(),
    );
    assert_eq!(report.summary.total_selectors, 1);
    assert_eq!(report.summary.uncovered_selectors, 1);
    assert_eq!(report.selectors[0].file, "web/app/page.tsx");
}

#[test]
fn selector_coverage_sort_uses_value_and_file_tiebreakers() {
    let root = Path::new("/repo");
    let app_selectors = vec![
        selectors::AppSelector {
            file: PathBuf::from("/repo/web/app/b.tsx"),
            attribute: "data-testid".to_string(),
            value: AppSelectorValue::Exact("same".to_string()),
        },
        selectors::AppSelector {
            file: PathBuf::from("/repo/web/app/a.tsx"),
            attribute: "data-testid".to_string(),
            value: AppSelectorValue::Exact("same".to_string()),
        },
        selectors::AppSelector {
            file: PathBuf::from("/repo/web/app/c.tsx"),
            attribute: "data-testid".to_string(),
            value: AppSelectorValue::Exact("zzz".to_string()),
        },
    ];
    let settings = default_settings(vec!["data-testid".to_string()]);
    let report = build_coverage(
        root,
        &[],
        &app_selectors,
        &app_selectors,
        &[],
        &settings,
        UniqueSelectorPolicy::default(),
    );
    assert_eq!(report.selectors[0].file, "web/app/a.tsx");
    assert_eq!(report.selectors[1].file, "web/app/b.tsx");
    assert_eq!(report.selectors[2].value, "zzz");
}

#[test]
fn selector_edges_mark_targets_covered() {
    let root = Path::new("/repo");
    let app_selectors = vec![selectors::AppSelector {
        file: PathBuf::from("/repo/web/app/page.tsx"),
        attribute: "data-testid".to_string(),
        value: AppSelectorValue::Exact("save".to_string()),
    }];
    let edges = vec![Edge::Selector {
        test_file: "tests/e2e/app.spec.ts".to_string(),
        test_name: None,
        describe_path: vec![],
        app_file: "web/app/page.tsx".to_string(),
        attribute: "data-testid".to_string(),
        value: "save".to_string(),
        selector: "getByTestId(save)".to_string(),
    }];
    let settings = default_settings(vec!["data-testid".to_string()]);
    let report = build_coverage(
        root,
        &[],
        &app_selectors,
        &app_selectors,
        &edges,
        &settings,
        UniqueSelectorPolicy::default(),
    );
    assert_eq!(report.summary.covered_selectors, 1);
    assert_eq!(report.selectors[0].tests, vec!["tests/e2e/app.spec.ts"]);
}

#[test]
fn route_edges_mark_routes_covered() {
    let root = Path::new("/repo");
    let routes = vec![Route {
        file: PathBuf::from("/repo/web/app/users/[id]/page.tsx"),
        pattern: "/users/:id".to_string(),
    }];
    let edges = vec![Edge::Route {
        test_file: "tests/e2e/users.spec.ts".to_string(),
        test_name: None,
        describe_path: vec![],
        route_file: "web/app/users/[id]/page.tsx".to_string(),
        route: "/users/:id".to_string(),
        url: "/users/42".to_string(),
    }];
    let settings = default_settings(vec!["data-testid".to_string()]);
    let report = build_coverage(
        root,
        &routes,
        &[],
        &[],
        &edges,
        &settings,
        UniqueSelectorPolicy::default(),
    );
    assert_eq!(report.summary.covered_routes, 1);
    assert_eq!(report.routes[0].urls, vec!["/users/42"]);
}
