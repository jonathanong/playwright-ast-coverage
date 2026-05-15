use crate::analysis::app_collect::collect_app_selectors;
use crate::config::Settings;
use crate::fsutil::{build_globset, relative_string, walk_files};
use crate::selectors;
use crate::test_support::fixture_path;
use std::collections::BTreeMap;

#[test]
fn skipped_directories_are_detected() {
    use crate::fsutil::is_skipped_dir;
    use std::path::Path;
    assert!(is_skipped_dir(Path::new("node_modules")));
    assert!(!is_skipped_dir(Path::new("src")));
}

#[test]
fn build_globset_rejects_invalid_patterns() {
    assert!(build_globset(&["[".to_string()]).is_err());
}

#[test]
fn walk_files_returns_files_and_skips_configured_directories() {
    let root = fixture_path(&["main", "walk-files"]);
    let files: Vec<String> = walk_files(&root)
        .into_iter()
        .map(|path| relative_string(&root, &path))
        .collect();
    assert_eq!(files, vec!["src/a.ts", "src/b.ts"]);
}

#[test]
fn collect_app_selectors_skips_missing_roots_and_non_source_files() {
    let root = fixture_path(&["main", "selector-source"]);
    let settings = Settings {
        frontend_root: "web/app".to_string(),
        playwright_configs: vec![],
        project: None,
        test_include: vec![],
        test_exclude: vec![],
        ignore_routes: vec![],
        navigation_helpers: vec![],
        selector_attributes: vec!["data-testid".to_string()],
        component_selector_attributes: BTreeMap::new(),
        html_ids: false,
        selector_roots: vec!["missing".to_string(), "web/app".to_string()],
        selector_include: vec![],
        selector_exclude: vec![],
    };

    let selector_regexes = selectors::compile_selector_regexes(
        &settings.selector_attributes,
        &settings.component_selector_attributes,
    );
    let selectors = collect_app_selectors(&root, &settings, &selector_regexes).unwrap();
    assert_eq!(selectors.len(), 1);
    assert_eq!(selectors[0].display_value(), "save");
}
