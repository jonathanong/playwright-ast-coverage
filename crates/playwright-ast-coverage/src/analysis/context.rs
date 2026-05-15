use crate::playwright_config;
use crate::playwright_tests;
use crate::selectors;
use globset::GlobSet;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(crate) struct RouteTarget {
    pub(crate) route_file: String,
    pub(crate) pattern: String,
    pub(crate) segments: Vec<String>,
}

pub(crate) struct AppSelectorTarget<'a> {
    pub(crate) selector: &'a selectors::AppSelector,
    pub(crate) app_file: String,
    pub(crate) value: String,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct TestProjectContext {
    pub(crate) base_url: Option<String>,
    pub(crate) test_id_attribute: String,
}

pub(crate) struct DiscoveredTestFile {
    pub(crate) path: PathBuf,
    pub(crate) contexts: Vec<TestProjectContext>,
}

pub(crate) struct TestProjectDiscovery {
    pub(crate) context: TestProjectContext,
    pub(crate) test_dir: PathBuf,
    pub(crate) include: GlobSet,
    pub(crate) ignore: GlobSet,
}

pub(crate) struct TestAnalysisContext<'a> {
    pub(crate) root: &'a Path,
    pub(crate) route_index: &'a RouteIndex,
    pub(crate) app_selector_targets: &'a [AppSelectorTarget<'a>],
    pub(crate) selector_index: &'a SelectorIndex<'a>,
    pub(crate) navigation_helpers: &'a [String],
    pub(crate) selector_regexes: &'a selectors::SelectorRegexes,
    pub(crate) test_policy: playwright_tests::TestPolicy,
}

#[derive(Default)]
pub(crate) struct RouteIndex {
    pub(crate) root: Vec<RouteTarget>,
    pub(crate) literal_first: HashMap<String, Vec<RouteTarget>>,
    pub(crate) dynamic_first: Vec<RouteTarget>,
}

#[derive(Default)]
pub(crate) struct SelectorIndex<'a> {
    pub(crate) exact: HashMap<String, HashMap<String, Vec<&'a AppSelectorTarget<'a>>>>,
    pub(crate) by_attribute: HashMap<String, Vec<&'a AppSelectorTarget<'a>>>,
    pub(crate) templates_by_attribute: HashMap<String, Vec<&'a AppSelectorTarget<'a>>>,
}

impl TestProjectContext {
    pub(crate) fn from_project(project: &playwright_config::TestProject) -> Self {
        Self {
            base_url: project.base_url.clone(),
            test_id_attribute: project.test_id_attribute.clone(),
        }
    }
}

impl DiscoveredTestFile {
    pub(crate) fn base_urls(&self) -> Vec<String> {
        let mut urls: Vec<String> = self
            .contexts
            .iter()
            .filter_map(|context| context.base_url.clone())
            .collect();
        urls.sort();
        urls.dedup();
        urls
    }

    pub(crate) fn test_id_attributes(&self) -> Vec<String> {
        let mut attributes: Vec<String> = self
            .contexts
            .iter()
            .map(|context| context.test_id_attribute.clone())
            .collect();
        attributes.sort();
        attributes.dedup();
        attributes
    }
}
