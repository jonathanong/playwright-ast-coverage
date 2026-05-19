use crate::analysis::coverage::build_coverage;
use crate::analysis::types::{Edge, FetchIndex, UniqueSelectorPolicy};
use crate::config::Settings;
use crate::routes::Route;
use crate::selectors::{self, AppSelectorValue};
use no_mistakes_core::fetch::types::{CacheKind, FetchOccurrence, FetchSide};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[test]
fn fetch_edges_mark_fetch_apis_covered() {
    let root = Path::new("/repo");
    let routes = vec![Route {
        file: PathBuf::from("/repo/web/app/page.tsx"),
        pattern: "/".to_string(),
    }];
    let edges = vec![
        Edge::Fetch {
            test_file: "tests/e2e/app.spec.ts".to_string(),
            test_name: Some("visits home".to_string()),
            describe_path: vec!["Suite".to_string()],
            route_file: "web/app/page.tsx".to_string(),
            route: "/".to_string(),
            method: "GET".to_string(),
            path: "/api/health".to_string(),
            side: "server".to_string(),
            cached: false,
        },
        Edge::Fetch {
            test_file: "tests/e2e/app.spec.ts".to_string(),
            test_name: None,
            describe_path: vec![],
            route_file: "web/app/page.tsx".to_string(),
            route: "/".to_string(),
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            side: "client".to_string(),
            cached: true,
        },
    ];
    let settings = default_settings(vec![]);
    let report = build_coverage(
        root,
        &routes,
        &[],
        &[],
        &edges,
        &settings,
        UniqueSelectorPolicy::default(),
        &FetchIndex::new(),
    );
    assert_eq!(report.summary.total_fetch_apis, 2);
    assert_eq!(report.summary.covered_fetch_apis, 2);
    assert_eq!(report.summary.uncovered_fetch_apis, 0);
    // Sort order: GET /api/health before GET /api/users
    assert_eq!(report.fetch_apis[0].path, "/api/health");
    assert_eq!(report.fetch_apis[1].path, "/api/users");
    assert!(report.fetch_apis[0].covered);
    assert_eq!(report.fetch_apis[0].route_files, vec!["web/app/page.tsx"]);
}

#[test]
fn has_configured_html_id_via_component_attributes() {
    use crate::config::has_configured_html_id_selector;
    use crate::selectors::HTML_ID_ATTRIBUTE;
    let settings_with_component_id = Settings {
        frontend_root: "web/app".to_string(),
        playwright_configs: vec![],
        project: None,
        test_include: vec![],
        test_exclude: vec![],
        ignore_routes: vec![],
        navigation_helpers: vec![],
        selector_attributes: vec![],
        component_selector_attributes: std::collections::BTreeMap::from([(
            "ButtonId".to_string(),
            HTML_ID_ATTRIBUTE.to_string(),
        )]),
        html_ids: false,
        selector_roots: vec![],
        selector_include: vec![],
        selector_exclude: vec![],
    };
    assert!(has_configured_html_id_selector(&settings_with_component_id));
}

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
        &FetchIndex::new(),
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
        &FetchIndex::new(),
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
        &FetchIndex::new(),
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
        &FetchIndex::new(),
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
        &FetchIndex::new(),
    );
    assert_eq!(report.summary.covered_routes, 1);
    assert_eq!(report.routes[0].urls, vec!["/users/42"]);
}

#[test]
fn seed_fetch_coverage_skips_dynamic_and_unsupported() {
    let root = Path::new("/repo");
    let mut fetch_index = FetchIndex::new();
    fetch_index.insert(
        "web/app/page.tsx".to_string(),
        vec![
            FetchOccurrence {
                method: "GET".to_string(),
                path: "/api/data".to_string(),
                raw_path: "/api/data".to_string(),
                file: "web/app/page.tsx".to_string(),
                line: 1,
                side: FetchSide::Server,
                rsc: true,
                cached: false,
                cache_kind: CacheKind::None,
                cached_function: None,
                dynamic: true,
                unsupported: false,
            },
            FetchOccurrence {
                method: "GET".to_string(),
                path: "/api/static".to_string(),
                raw_path: "/api/static".to_string(),
                file: "web/app/page.tsx".to_string(),
                line: 2,
                side: FetchSide::Server,
                rsc: true,
                cached: false,
                cache_kind: CacheKind::None,
                cached_function: None,
                dynamic: false,
                unsupported: false,
            },
        ],
    );
    let settings = default_settings(vec![]);
    let report = build_coverage(
        root,
        &[],
        &[],
        &[],
        &[],
        &settings,
        UniqueSelectorPolicy::default(),
        &fetch_index,
    );
    assert_eq!(report.summary.total_fetch_apis, 1);
    assert_eq!(report.fetch_apis[0].path, "/api/static");
    assert!(!report.fetch_apis[0].covered);
}
