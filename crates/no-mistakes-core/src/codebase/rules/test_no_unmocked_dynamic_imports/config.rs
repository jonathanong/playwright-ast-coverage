mod discovery;
mod rule_targets;

use crate::config::v2::NoMistakesConfig;
use anyhow::Result;
use discovery::{
    build_globset, build_regexes, config_files, extract_property_strings,
    extract_test_property_strings, extract_test_regexes,
};
use globset::GlobSet;
use regex::Regex;
use std::path::{Path, PathBuf};

use rule_targets::rule_test_project_globs;

pub struct TestFilter {
    include: GlobSet,
    include_regex: Vec<Regex>,
    exclude: GlobSet,
}

pub struct ConfigSetupData {
    filter: TestFilter,
    pub setup_files: Vec<PathBuf>,
}

impl ConfigSetupData {
    pub fn filter_matches(&self, rel_path: &str) -> bool {
        self.filter.is_match(rel_path)
    }
}

impl TestFilter {
    pub fn is_match(&self, rel_path: &str) -> bool {
        let mut included = self.include.is_match(rel_path);
        if !included {
            for regex in &self.include_regex {
                if regex.is_match(rel_path) {
                    included = true;
                    break;
                }
            }
        }
        included && !self.exclude.is_match(rel_path)
    }
}

pub fn test_filter(root: &Path, config: &NoMistakesConfig) -> Result<TestFilter> {
    let (mut includes, mut excludes) = rule_test_project_globs(root, config)?;
    let has_rule_target_includes = !includes.is_empty();
    let mut include_regex = Vec::new();
    let mut config_includes = Vec::new();
    for config_file in config_files(root, config) {
        let source = std::fs::read_to_string(&config_file.path)?;
        let base = config_file.path.parent().unwrap_or(root);
        config_includes.extend(normalize_matcher_patterns(
            root,
            base,
            extract_test_property_strings(&source, "include"),
        ));
        config_includes.extend(normalize_matcher_patterns(
            root,
            base,
            extract_property_strings(&source, "testMatch"),
        ));
        include_regex.extend(extract_test_regexes(&source));
        excludes.extend(normalize_matcher_patterns(
            root,
            base,
            extract_test_property_strings(&source, "exclude"),
        ));
    }
    if has_rule_target_includes {
        include_regex.clear();
    } else if !config_includes.is_empty() || !include_regex.is_empty() {
        includes = config_includes;
    } else {
        includes = crate::codebase::dependencies::VITEST_JEST_TEST_GLOBS
            .iter()
            .map(|s| (*s).to_string())
            .collect::<Vec<_>>();
    }
    Ok(TestFilter {
        include: build_globset(&includes)?,
        include_regex: build_regexes(&include_regex)?,
        exclude: build_globset(&excludes)?,
    })
}

/// Pre-compute per-config filter and setup files once, so the per-test loop can skip
/// re-reading and re-parsing config files on every iteration.
pub fn precompute_setup_data(
    root: &Path,
    config: &NoMistakesConfig,
) -> Result<Vec<ConfigSetupData>> {
    let mut result = Vec::new();
    for config_file in config_files(root, config) {
        let source = std::fs::read_to_string(&config_file.path)?;
        let base = config_file.path.parent().unwrap_or(root);
        let includes = normalize_matcher_patterns(root, base, config_file.includes(&source));
        let excludes = normalize_matcher_patterns(
            root,
            base,
            extract_test_property_strings(&source, "exclude"),
        );
        let filter = TestFilter {
            include: build_globset(&includes)?,
            include_regex: build_regexes(&extract_test_regexes(&source))?,
            exclude: build_globset(&excludes)?,
        };
        let setup_files = setup_files_from_configs(root, vec![config_file.path])?;
        result.push(ConfigSetupData {
            filter,
            setup_files,
        });
    }
    Ok(result)
}

pub fn setup_files_for_test_precomputed(
    rel_path: &str,
    config_data: &[ConfigSetupData],
) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for data in config_data {
        if data.filter_matches(rel_path) {
            files.extend(data.setup_files.iter().cloned());
        }
    }
    files.sort();
    files.dedup();
    files
}

fn normalize_matcher_patterns(root: &Path, base: &Path, patterns: Vec<String>) -> Vec<String> {
    patterns
        .into_iter()
        .map(|pattern| normalize_matcher_pattern(root, base, pattern))
        .collect()
}

fn normalize_matcher_pattern(root: &Path, base: &Path, pattern: String) -> String {
    if pattern == "<rootDir>" {
        return crate::codebase::ts_source::relative_slash_path(root, base);
    }
    if let Some(rest) = pattern.strip_prefix("<rootDir>/") {
        return crate::codebase::ts_source::relative_slash_path(root, &base.join(rest));
    }
    if let Some(rest) = pattern.strip_prefix("./") {
        return crate::codebase::ts_source::relative_slash_path(root, &base.join(rest));
    }
    pattern
}

fn setup_files_from_configs(root: &Path, config_files: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for config_file in config_files {
        let source = std::fs::read_to_string(&config_file)?;
        let base = config_file.parent().unwrap_or(root);
        let mut setups = extract_test_property_strings(&source, "setupFiles");
        setups.extend(extract_property_strings(&source, "setupFiles"));
        setups.extend(extract_property_strings(&source, "setupFilesAfterEnv"));
        for setup in setups {
            let path = resolve_setup_file(base, &setup);
            if path.exists() {
                files.push(crate::codebase::ts_resolver::normalize_path(&path));
            }
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn resolve_setup_file(base: &Path, setup: &str) -> PathBuf {
    if setup == "<rootDir>" {
        return base.to_path_buf();
    }
    if let Some(rest) = setup.strip_prefix("<rootDir>/") {
        return base.join(rest);
    }
    crate::config::resolve(base, Path::new(setup))
}

#[cfg(test)]
mod tests;
