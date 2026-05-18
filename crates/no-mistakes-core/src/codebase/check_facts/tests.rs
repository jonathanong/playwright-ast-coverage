use super::{collect_check_facts, CheckFactPlan};
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis/shared-facts")
        .join(name)
}

#[test]
fn collect_check_facts_records_read_errors() {
    let root = fixture_path("");
    let file = fixture_path("src/unreadable.ts");
    let facts = collect_check_facts(
        &root,
        vec![file.clone()],
        CheckFactPlan {
            source: true,
            ..CheckFactPlan::default()
        },
    );

    assert_eq!(facts.stats.parse_errors, 1);
    let file_facts = facts.ts.get(&file).expect("read error fact is recorded");
    assert!(file_facts
        .parse_error
        .as_deref()
        .is_some_and(|error| error.contains("failed to read")));
}

#[test]
fn collect_check_facts_skips_non_indexable_files_with_minimal_plan() {
    let root = fixture_path("");
    let file = fixture_path("src/everything.tsx");
    let non_indexable = fixture_path("README.md");
    let facts = collect_check_facts(
        &root,
        vec![file.clone(), non_indexable],
        CheckFactPlan::default(),
    );

    assert_eq!(facts.stats.files_discovered, 2);
    assert_eq!(facts.stats.files_parsed, 1);
    assert_eq!(facts.stats.parse_errors, 0);
    assert_eq!(facts.ts.len(), 1);
    let file_facts = facts.ts.get(&file).expect("indexable file is parsed");
    assert!(file_facts.imports.is_empty());
    assert!(file_facts.symbols.is_none());
    assert!(file_facts.source.is_none());
}

#[test]
fn collect_check_facts_records_parse_error_details() {
    let root = fixture_path("");
    let file = fixture_path("src/invalid.ts");
    let facts = collect_check_facts(
        &root,
        vec![file.clone()],
        CheckFactPlan {
            source: true,
            ..CheckFactPlan::default()
        },
    );
    let parse_error = facts
        .ts
        .get(&file)
        .and_then(|facts| facts.parse_error.as_ref())
        .expect("parse error is recorded");

    assert_eq!(facts.stats.parse_errors, 1);
    assert_ne!(parse_error, &file.display().to_string());
    assert!(!parse_error.is_empty());
    let file_facts = facts.ts.get(&file).expect("file facts are retained");
    assert!(file_facts.source.is_some());
    assert!(file_facts.imports.is_empty());
    assert!(file_facts.symbols.is_none());
}

#[test]
fn collect_check_facts_parses_once_for_overlapping_fact_categories() {
    let root = fixture_path("");
    let file = fixture_path("src/everything.tsx");
    let facts = collect_check_facts(
        &root,
        vec![file.clone()],
        CheckFactPlan {
            imports: true,
            symbols: true,
            react: true,
            queue: true,
            integration: true,
            dynamic_imports: true,
            source: true,
        },
    );

    assert_eq!(facts.stats.files_discovered, 1);
    assert_eq!(facts.stats.files_parsed, 1);
    assert_eq!(facts.stats.parse_errors, 0);
    let file_facts = facts.ts.get(&file).expect("file facts are collected");
    assert!(!file_facts.imports.is_empty());
    assert!(file_facts.symbols.is_some());
    assert!(file_facts.react.is_some());
    assert!(file_facts.queue.is_some());
    assert!(file_facts.integration.is_some());
    assert!(file_facts.dynamic_imports.is_some());
    assert!(file_facts.source.is_some());
}
