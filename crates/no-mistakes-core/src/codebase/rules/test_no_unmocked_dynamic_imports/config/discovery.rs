mod property_strings;
mod regex_literals;

use crate::config::v2::{ConfigView, NoMistakesConfig};
use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};
pub(super) use property_strings::extract_property_strings;
use regex::Regex;
use regex_literals::extract_test_regex_literals;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub(super) enum Runner {
    Vitest,
    Jest,
}

pub(super) struct ConfigFile {
    pub path: PathBuf,
    pub(super) runner: Runner,
}

impl ConfigFile {
    pub fn includes(&self, source: &str) -> Vec<String> {
        let mut includes = match self.runner {
            Runner::Vitest => extract_test_property_strings(source, "include"),
            Runner::Jest => extract_property_strings(source, "testMatch"),
        };
        if includes.is_empty() && !self.has_configured_matcher(source) {
            for pattern in crate::codebase::dependencies::VITEST_JEST_TEST_GLOBS {
                includes.push((*pattern).to_string());
            }
        }
        includes
    }

    fn has_configured_matcher(&self, source: &str) -> bool {
        match self.runner {
            Runner::Vitest => !extract_test_property_strings(source, "include").is_empty(),
            Runner::Jest => {
                !extract_property_strings(source, "testMatch").is_empty()
                    || !extract_test_regexes(source).is_empty()
            }
        }
    }
}

pub(super) fn config_files(root: &Path, config: &NoMistakesConfig) -> Vec<ConfigFile> {
    let view = ConfigView::new(config);
    let mut configured = expand_config_patterns(
        root,
        view.vitest_configs().unwrap_or_default(),
        Runner::Vitest,
    );
    configured.extend(expand_config_patterns(
        root,
        view.jest_configs().unwrap_or_default(),
        Runner::Jest,
    ));
    let configured = normalize_configs(configured);
    if !configured.is_empty() {
        return configured;
    }
    discovered_configs(root)
}

fn discovered_configs(root: &Path) -> Vec<ConfigFile> {
    let discovered = [
        "vitest.config.ts",
        "vitest.config.mts",
        "vitest.config.js",
        "vitest.config.mjs",
    ]
    .into_iter()
    .map(|path| ConfigFile {
        path: root.join(path),
        runner: Runner::Vitest,
    })
    .chain(jest_config_names().into_iter().map(|path| ConfigFile {
        path: root.join(path),
        runner: Runner::Jest,
    }));
    normalize_configs(discovered.filter(|config| config.path.exists()).collect())
}

fn jest_config_names() -> [&'static str; 7] {
    [
        "jest.config.ts",
        "jest.config.mts",
        "jest.config.cts",
        "jest.config.js",
        "jest.config.mjs",
        "jest.config.cjs",
        "jest.config.json",
    ]
}

fn normalize_configs(configs: Vec<ConfigFile>) -> Vec<ConfigFile> {
    configs
        .into_iter()
        .map(|config| ConfigFile {
            path: crate::codebase::ts_resolver::normalize_path(&config.path),
            runner: config.runner,
        })
        .collect()
}

fn expand_config_patterns(root: &Path, patterns: Vec<String>, runner: Runner) -> Vec<ConfigFile> {
    if patterns.is_empty() {
        return Vec::new();
    }
    let has_globs = patterns.iter().any(|p| is_glob(p));
    let files = if has_globs {
        crate::codebase::ts_source::discover_files(root, &[])
    } else {
        Vec::new()
    };
    let mut configs = Vec::new();
    for pattern in patterns {
        if is_glob(&pattern) {
            if let Ok(glob) = Glob::new(&pattern) {
                let matcher = glob.compile_matcher();
                for file in &files {
                    let rel = crate::codebase::ts_source::relative_slash_path(root, file);
                    if matcher.is_match(rel) {
                        configs.push(ConfigFile {
                            path: file.clone(),
                            runner,
                        });
                    }
                }
            }
        } else {
            let path = root.join(pattern);
            if path.exists() {
                configs.push(ConfigFile { path, runner });
            }
        }
    }
    configs
}

fn is_glob(pattern: &str) -> bool {
    pattern.contains('*') || pattern.contains('?') || pattern.contains('[')
}

pub(super) fn build_globset(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
    }
    Ok(builder.build()?)
}

pub(super) fn build_regexes(patterns: &[String]) -> Result<Vec<Regex>> {
    let mut regexes = Vec::new();
    for pattern in patterns {
        regexes.push(Regex::new(pattern)?);
    }
    Ok(regexes)
}

pub(super) fn extract_test_property_strings(source: &str, property: &str) -> Vec<String> {
    find_property_object(source, "test")
        .map(|test| extract_property_strings(test, property))
        .unwrap_or_default()
}

pub(super) fn extract_test_regexes(source: &str) -> Vec<String> {
    let mut regexes = extract_property_strings(source, "testRegex");
    regexes.extend(extract_test_regex_literals(source));
    regexes
}

fn find_property_object<'a>(source: &'a str, property: &str) -> Option<&'a str> {
    let re = Regex::new(&format!(r#"\b{}\s*:\s*\{{"#, regex::escape(property)))
        .expect("object property regex compiles");
    let mat = re.find(source)?;
    let open = source[mat.start()..].find('{')? + mat.start();
    let close = matching_brace(source, open)?;
    source.get(open + 1..close)
}

fn matching_brace(source: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for (idx, ch) in source.char_indices().skip_while(|(idx, _)| *idx < open) {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' | '`' => quote = Some(ch),
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}
