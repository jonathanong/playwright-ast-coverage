use crate::config::v2::{ConfigView, NoMistakesConfig};
use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::Regex;
use std::path::{Path, PathBuf};

pub struct TestFilter {
    include: GlobSet,
    exclude: GlobSet,
}

impl TestFilter {
    pub fn is_match(&self, rel_path: String) -> bool {
        self.include.is_match(&rel_path) && !self.exclude.is_match(&rel_path)
    }
}

pub fn test_filter(root: &Path, config: &NoMistakesConfig) -> Result<TestFilter> {
    let mut includes = crate::codebase::dependencies::VITEST_JEST_TEST_GLOBS
        .iter()
        .map(|s| (*s).to_string())
        .collect::<Vec<_>>();
    let mut excludes = Vec::new();
    for config_file in config_files(root, config) {
        if let Ok(source) = std::fs::read_to_string(&config_file) {
            includes.extend(extract_property_strings(&source, "include"));
            excludes.extend(extract_property_strings(&source, "exclude"));
        }
    }
    Ok(TestFilter {
        include: build_globset(&includes)?,
        exclude: build_globset(&excludes)?,
    })
}

pub fn setup_files(root: &Path, config: &NoMistakesConfig) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for config_file in config_files(root, config) {
        let source = std::fs::read_to_string(&config_file)?;
        let base = config_file.parent().unwrap_or(root);
        for setup in extract_property_strings(&source, "setupFiles") {
            let path = crate::config::resolve(base, Path::new(&setup));
            if path.exists() {
                files.push(crate::codebase::ts_resolver::normalize_path(&path));
            }
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn config_files(root: &Path, config: &NoMistakesConfig) -> Vec<PathBuf> {
    let view = ConfigView::new(config);
    let configured = view
        .vitest_configs()
        .into_iter()
        .flatten()
        .chain(view.jest_configs().into_iter().flatten())
        .map(|path| root.join(path));
    let discovered = [
        "vitest.config.ts",
        "vitest.config.mts",
        "vitest.config.js",
        "vitest.config.mjs",
        "jest.config.ts",
        "jest.config.mts",
        "jest.config.js",
        "jest.config.mjs",
    ]
    .into_iter()
    .map(|path| root.join(path));
    configured
        .chain(discovered)
        .filter(|path| path.exists())
        .map(|path| crate::codebase::ts_resolver::normalize_path(&path))
        .collect()
}

fn build_globset(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
    }
    Ok(builder.build()?)
}

fn extract_property_strings(source: &str, property: &str) -> Vec<String> {
    let re = Regex::new(&format!(
        r#"(?s)\b{}\s*:\s*(\[[^\]]*\]|['"][^'"]+['"])"#,
        regex::escape(property)
    ))
    .expect("property regex compiles");
    let string_re = Regex::new(r#"['"]([^'"]+)['"]"#).expect("string regex compiles");
    re.captures_iter(source)
        .flat_map(|capture| {
            string_re
                .captures_iter(&capture[1])
                .filter_map(|string| string.get(1).map(|m| m.as_str().to_string()))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(test)]
mod tests;
