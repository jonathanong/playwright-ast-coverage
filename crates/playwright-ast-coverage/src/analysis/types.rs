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
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoverageRoute {
    pub(crate) route: String,
    pub(crate) file: String,
    pub(crate) covered: bool,
    pub(crate) tests: Vec<String>,
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
    pub(crate) selectors: Vec<String>,
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
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum Edge {
    #[serde(rename_all = "camelCase")]
    Route {
        test_file: String,
        route_file: String,
        route: String,
        url: String,
    },
    #[serde(rename_all = "camelCase")]
    Selector {
        test_file: String,
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

pub(crate) type SelectorCoverageKey = (String, String, String);
pub(crate) type CoverageLinks = (
    std::collections::BTreeSet<String>,
    std::collections::BTreeSet<String>,
);
