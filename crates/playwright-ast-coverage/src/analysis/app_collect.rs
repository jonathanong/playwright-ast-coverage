use crate::config::Settings;
use crate::fsutil::{build_globset, relative_string, walk_files};
use crate::selectors;
use anyhow::Result;
use rayon::prelude::*;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(crate) fn collect_app_selector_occurrences(
    root: &Path,
    settings: &Settings,
    selector_regexes: &selectors::SelectorRegexes,
) -> Result<Vec<selectors::AppSelector>> {
    let include = build_globset(&settings.selector_include)?;
    let exclude = build_globset(&settings.selector_exclude)?;
    let include_all = settings.selector_include.is_empty();
    let source_files =
        collect_selector_source_files(root, settings, &include, &exclude, include_all);
    let app_selectors = source_files
        .par_iter()
        .try_fold(Vec::new, |mut app_selectors, path| -> Result<_> {
            let source = std::fs::read_to_string(path)?;
            app_selectors.extend(selectors::extract_app_selectors_with_regexes(
                path,
                &source,
                selector_regexes,
            )?);
            Ok(app_selectors)
        })
        .try_reduce(Vec::new, |mut left, mut right| -> Result<_> {
            left.append(&mut right);
            Ok(left)
        })?;
    Ok(app_selectors)
}

pub(crate) fn collect_selector_source_files(
    root: &Path,
    settings: &Settings,
    include: &globset::GlobSet,
    exclude: &globset::GlobSet,
    include_all: bool,
) -> Vec<PathBuf> {
    let mut source_files = BTreeSet::new();
    for selector_root in &settings.selector_roots {
        let source_root = root.join(selector_root);
        if !source_root.exists() {
            continue;
        }

        for path in walk_files(&source_root) {
            if !selectors::is_source_file(&path) {
                continue;
            }
            let rel = relative_string(root, &path);
            if (!include_all && !include.is_match(&rel)) || exclude.is_match(&rel) {
                continue;
            }

            source_files.insert(path);
        }
    }

    source_files.into_iter().collect()
}

#[cfg(test)]
pub(crate) fn collect_app_selectors(
    root: &Path,
    settings: &Settings,
    selector_regexes: &selectors::SelectorRegexes,
) -> Result<Vec<selectors::AppSelector>> {
    let mut app_selectors = collect_app_selector_occurrences(root, settings, selector_regexes)?;
    app_selectors.sort();
    app_selectors.dedup();
    Ok(app_selectors)
}
