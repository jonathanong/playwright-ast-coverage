use super::*;

#[test]
fn source_files_from_facts_skips_disabled_parse_errors() {
    let root = fixture("unique-exports-edge-cases");
    let file = root.join("src/disabled-invalid.ts");
    let files = vec![file];
    let facts = crate::codebase::check_facts::collect_check_facts(
        &root,
        files.clone(),
        crate::codebase::check_facts::CheckFactPlan {
            symbols: true,
            source: true,
            ..Default::default()
        },
    );

    let source_files = scan::collect_source_files_from_facts(&root, &files, &facts).unwrap();

    assert_eq!(source_files.len(), 1);
    assert!(source_files[0].disabled);
    assert!(source_files[0].symbols.exports.is_empty());
}
