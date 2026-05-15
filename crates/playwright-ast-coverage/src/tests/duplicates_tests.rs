use crate::analysis::duplicates::build_duplicate_selectors;
use crate::analysis::types::UniqueSelectorPolicy;
use crate::selectors::{self, AppSelectorValue};
use std::path::{Path, PathBuf};

#[test]
fn duplicate_selector_report_includes_exact_values_only() {
    let root = Path::new("/repo");
    let app_selectors = vec![
        selectors::AppSelector {
            file: PathBuf::from("/repo/web/app/b.tsx"),
            attribute: "data-pw".to_string(),
            value: AppSelectorValue::Exact("same".to_string()),
        },
        selectors::AppSelector {
            file: PathBuf::from("/repo/web/app/b.tsx"),
            attribute: "data-pw".to_string(),
            value: AppSelectorValue::Exact("same".to_string()),
        },
        selectors::AppSelector {
            file: PathBuf::from("/repo/web/app/b.tsx"),
            attribute: "data-testid".to_string(),
            value: AppSelectorValue::Exact("same".to_string()),
        },
        selectors::AppSelector {
            file: PathBuf::from("/repo/web/app/a.tsx"),
            attribute: "data-testid".to_string(),
            value: AppSelectorValue::Exact("same".to_string()),
        },
        selectors::AppSelector {
            file: PathBuf::from("/repo/web/app/c.tsx"),
            attribute: "data-testid".to_string(),
            value: AppSelectorValue::Unsupported("id".to_string()),
        },
        selectors::AppSelector {
            file: PathBuf::from("/repo/web/app/d.tsx"),
            attribute: "data-testid".to_string(),
            value: AppSelectorValue::Exact("unique".to_string()),
        },
    ];

    let duplicates = build_duplicate_selectors(
        root,
        &app_selectors,
        UniqueSelectorPolicy {
            test_ids: true,
            html_ids: false,
            ..UniqueSelectorPolicy::default()
        },
    );
    assert_eq!(duplicates.len(), 4);
    assert_eq!(duplicates[0].file, "web/app/a.tsx");
    assert_eq!(duplicates[1].attribute, "data-pw");
    assert_eq!(duplicates[2].attribute, "data-pw");
    assert_eq!(duplicates[3].attribute, "data-testid");
}

#[test]
fn duplicate_selector_report_keeps_html_ids_separate_from_test_ids() {
    let root = Path::new("/repo");
    let app_selectors = vec![
        selectors::AppSelector {
            file: PathBuf::from("/repo/web/app/a.tsx"),
            attribute: "data-testid".to_string(),
            value: AppSelectorValue::Exact("same".to_string()),
        },
        selectors::AppSelector {
            file: PathBuf::from("/repo/web/app/b.tsx"),
            attribute: "id".to_string(),
            value: AppSelectorValue::Exact("same".to_string()),
        },
    ];

    let duplicates = build_duplicate_selectors(
        root,
        &app_selectors,
        UniqueSelectorPolicy {
            test_ids: true,
            html_ids: true,
            ..UniqueSelectorPolicy::default()
        },
    );
    assert!(duplicates.is_empty());
}

#[test]
fn deprecated_duplicate_selector_report_preserves_aggregate_grouping() {
    let root = Path::new("/repo");
    let app_selectors = vec![
        selectors::AppSelector {
            file: PathBuf::from("/repo/web/app/a.tsx"),
            attribute: "data-testid".to_string(),
            value: AppSelectorValue::Exact("same".to_string()),
        },
        selectors::AppSelector {
            file: PathBuf::from("/repo/web/app/b.tsx"),
            attribute: "id".to_string(),
            value: AppSelectorValue::Exact("same".to_string()),
        },
    ];

    let duplicates = build_duplicate_selectors(
        root,
        &app_selectors,
        UniqueSelectorPolicy {
            test_ids: true,
            html_ids: true,
            aggregate: true,
            ..UniqueSelectorPolicy::default()
        },
    );
    assert_eq!(duplicates.len(), 2);
}

#[test]
fn configured_html_id_selectors_count_as_test_ids_for_uniqueness() {
    let root = Path::new("/repo");
    let app_selectors = vec![
        selectors::AppSelector {
            file: PathBuf::from("/repo/web/app/a.tsx"),
            attribute: "id".to_string(),
            value: AppSelectorValue::Exact("same".to_string()),
        },
        selectors::AppSelector {
            file: PathBuf::from("/repo/web/app/b.tsx"),
            attribute: "id".to_string(),
            value: AppSelectorValue::Exact("same".to_string()),
        },
    ];

    let duplicates = build_duplicate_selectors(
        root,
        &app_selectors,
        UniqueSelectorPolicy {
            test_ids: true,
            configured_html_id_selector: true,
            ..UniqueSelectorPolicy::default()
        },
    );
    assert_eq!(duplicates.len(), 2);
}
