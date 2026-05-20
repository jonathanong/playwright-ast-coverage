use crate::config::v2::NoMistakesConfig;
use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::Path;

type RunnerTestFilter =
    crate::codebase::rules::test_no_unmocked_dynamic_imports::config::TestFilter;

#[derive(Clone)]
pub(crate) struct TestFileFilter {
    config_filter: Option<RunnerTestFilter>,
    suites: Vec<TestSuiteFilter>,
}

#[derive(Clone)]
struct TestSuiteFilter {
    include: Option<GlobSet>,
    exclude: Option<GlobSet>,
}

impl TestFileFilter {
    pub(crate) fn new(root: &Path, config: &NoMistakesConfig) -> Self {
        Self {
            config_filter:
                crate::codebase::rules::test_no_unmocked_dynamic_imports::config::test_filter(
                    root, config,
                )
                .ok(),
            suites: configured_suite_filters(config),
        }
    }

    pub(crate) fn is_match(&self, root: &Path, path: &Path) -> bool {
        let rel = crate::codebase::ts_source::relative_slash_path(root, path);
        self.is_match_rel(&rel)
    }

    pub(crate) fn is_match_rel(&self, rel_path: &str) -> bool {
        if let Some(is_match) = self.configured_suite_match(rel_path) {
            return is_match;
        }
        if self
            .config_filter
            .as_ref()
            .is_some_and(|filter| filter.is_match(rel_path))
        {
            return true;
        }
        fallback_test_path(rel_path)
    }

    fn configured_suite_match(&self, rel_path: &str) -> Option<bool> {
        let mut excluded_by_matching_suite = false;
        for suite in &self.suites {
            if !suite.matches_include(rel_path) {
                continue;
            }
            if suite.matches_exclude(rel_path) {
                excluded_by_matching_suite = true;
            } else {
                return Some(true);
            }
        }
        excluded_by_matching_suite.then_some(false)
    }
}

impl TestSuiteFilter {
    fn matches_include(&self, rel_path: &str) -> bool {
        self.include
            .as_ref()
            .is_some_and(|include| include.is_match(rel_path))
    }

    fn matches_exclude(&self, rel_path: &str) -> bool {
        self.exclude
            .as_ref()
            .is_some_and(|exclude| exclude.is_match(rel_path))
    }
}

fn configured_suite_filters(config: &NoMistakesConfig) -> Vec<TestSuiteFilter> {
    config
        .tests
        .vitest
        .suites
        .iter()
        .chain(config.tests.playwright.suites.iter())
        .filter_map(|suite| {
            let include = compile_optional_globset(&suite.include).ok().flatten();
            include.as_ref()?;
            let exclude = compile_optional_globset(&suite.exclude).ok().flatten();
            Some(TestSuiteFilter { include, exclude })
        })
        .collect()
}

fn compile_optional_globset(patterns: &[String]) -> Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
    }
    Ok(Some(builder.build()?))
}

fn fallback_test_path(rel_path: &str) -> bool {
    rel_path
        .split('/')
        .any(|component| component == "__tests__")
        || rel_path
            .rsplit('/')
            .next()
            .is_some_and(|name| name.contains(".test.") || name.contains(".spec."))
}

#[cfg(test)]
mod tests;
