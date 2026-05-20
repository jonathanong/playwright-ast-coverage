use crate::config::v2::NoMistakesConfig;
use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::Path;

type RunnerTestFilter =
    crate::codebase::rules::test_no_unmocked_dynamic_imports::config::TestFilter;

#[derive(Clone)]
pub(crate) struct TestFileFilter {
    config_filter: Option<RunnerTestFilter>,
    suite_include: Option<GlobSet>,
    suite_exclude: Option<GlobSet>,
}

impl TestFileFilter {
    pub(crate) fn new(root: &Path, config: &NoMistakesConfig) -> Self {
        Self {
            config_filter:
                crate::codebase::rules::test_no_unmocked_dynamic_imports::config::test_filter(
                    root, config,
                )
                .ok(),
            suite_include: compile_optional_globset(&configured_suite_includes(config))
                .ok()
                .flatten(),
            suite_exclude: compile_optional_globset(&configured_suite_excludes(config))
                .ok()
                .flatten(),
        }
    }

    pub(crate) fn is_match(&self, root: &Path, path: &Path) -> bool {
        let rel = crate::codebase::ts_source::relative_slash_path(root, path);
        self.is_match_rel(&rel)
    }

    pub(crate) fn is_match_rel(&self, rel_path: &str) -> bool {
        if self.matches_configured_suite_exclude(rel_path) {
            return false;
        }
        self.config_filter
            .as_ref()
            .is_some_and(|filter| filter.is_match(rel_path))
            || self.matches_configured_suite(rel_path)
            || fallback_test_path(rel_path)
    }

    fn matches_configured_suite(&self, rel_path: &str) -> bool {
        self.suite_include
            .as_ref()
            .is_some_and(|include| include.is_match(rel_path))
    }

    fn matches_configured_suite_exclude(&self, rel_path: &str) -> bool {
        self.suite_exclude
            .as_ref()
            .is_some_and(|exclude| exclude.is_match(rel_path))
    }
}

fn configured_suite_includes(config: &NoMistakesConfig) -> Vec<String> {
    config
        .tests
        .vitest
        .suites
        .iter()
        .chain(config.tests.playwright.suites.iter())
        .flat_map(|suite| suite.include.iter().cloned())
        .collect()
}

fn configured_suite_excludes(config: &NoMistakesConfig) -> Vec<String> {
    config
        .tests
        .vitest
        .suites
        .iter()
        .chain(config.tests.playwright.suites.iter())
        .flat_map(|suite| suite.exclude.iter().cloned())
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
