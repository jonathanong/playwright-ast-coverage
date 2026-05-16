use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Summary {
    pub(crate) total_routes: usize,
    pub(crate) covered_routes: usize,
    pub(crate) uncovered_routes: usize,
    pub(crate) total_selectors: usize,
    pub(crate) covered_selectors: usize,
    pub(crate) uncovered_selectors: usize,
    pub(crate) duplicate_selectors: usize,
    pub(crate) total_fetch_apis: usize,
    pub(crate) covered_fetch_apis: usize,
    pub(crate) uncovered_fetch_apis: usize,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestRef {
    pub(crate) file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) describe_path: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoverageRoute {
    pub(crate) route: String,
    pub(crate) file: String,
    pub(crate) covered: bool,
    pub(crate) tests: Vec<String>,
    pub(crate) tests_detail: Vec<TestRef>,
    pub(crate) urls: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoverageSelector {
    pub(crate) attribute: String,
    pub(crate) value: String,
    pub(crate) file: String,
    pub(crate) covered: bool,
    pub(crate) unsupported_dynamic: bool,
    pub(crate) tests: Vec<String>,
    pub(crate) tests_detail: Vec<TestRef>,
    pub(crate) selectors: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoverageFetch {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) covered: bool,
    pub(crate) tests: Vec<String>,
    pub(crate) tests_detail: Vec<TestRef>,
    pub(crate) route_files: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuplicateSelector {
    pub(crate) attribute: String,
    pub(crate) value: String,
    pub(crate) file: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoverageReport {
    pub(crate) summary: Summary,
    pub(crate) routes: Vec<CoverageRoute>,
    pub(crate) selectors: Vec<CoverageSelector>,
    pub(crate) duplicate_selectors: Vec<DuplicateSelector>,
    pub(crate) fetch_apis: Vec<CoverageFetch>,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum Edge {
    #[serde(rename_all = "camelCase")]
    Fetch {
        test_file: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        test_name: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        describe_path: Vec<String>,
        route_file: String,
        route: String,
        method: String,
        path: String,
        side: String,
        cached: bool,
    },
    #[serde(rename_all = "camelCase")]
    Route {
        test_file: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        test_name: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        describe_path: Vec<String>,
        route_file: String,
        route: String,
        url: String,
    },
    #[serde(rename_all = "camelCase")]
    Selector {
        test_file: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        test_name: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        describe_path: Vec<String>,
        app_file: String,
        attribute: String,
        value: String,
        selector: String,
    },
}

#[derive(Serialize)]
pub(crate) struct EdgeReport {
    pub(crate) edges: Vec<Edge>,
}

#[derive(Serialize)]
pub(crate) struct RelatedReport {
    pub(crate) tests: Vec<String>,
    pub(crate) fetch_apis: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestsReport {
    pub(crate) tests: Vec<TestEntry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestEntry {
    pub(crate) file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) describe_path: Vec<String>,
    pub(crate) test_ids: Vec<String>,
    pub(crate) html_ids: Vec<String>,
    pub(crate) routes: Vec<String>,
    pub(crate) fetch_apis: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Analysis {
    pub(crate) coverage: CoverageReport,
    pub(crate) edges: EdgeReport,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct UniqueSelectorPolicy {
    pub(crate) test_ids: bool,
    pub(crate) html_ids: bool,
    pub(crate) aggregate: bool,
    pub(crate) configured_html_id_selector: bool,
}

pub(crate) type FetchIndex =
    std::collections::HashMap<String, Vec<no_mistakes_core::fetch::types::FetchOccurrence>>;
pub(crate) type SelectorCoverageKey = (String, String, String);
pub(crate) type CoverageLinks = (
    std::collections::BTreeSet<String>,
    std::collections::BTreeSet<String>,
);
