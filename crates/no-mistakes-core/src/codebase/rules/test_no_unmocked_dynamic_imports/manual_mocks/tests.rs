use super::*;

#[test]
fn maps_adjacent_and_rooted_manual_mocks_to_targets() {
    let root = PathBuf::from("/repo");
    let adjacent = root.join("src/__mocks__/manual.mts");
    let targets = mocked_targets(&root, &adjacent);
    assert!(targets.contains(&root.join("src/manual.mts")));
    assert!(targets.contains(&root.join("manual.mts")));

    let rooted = root.join("__mocks__/src/manual.mts");
    let targets = mocked_targets(&root, &rooted);
    assert!(targets.contains(&root.join("src/manual.mts")));
}

#[test]
fn discover_respects_skip_directories() {
    let root = crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/test-no-unmocked-dynamic-imports"),
    );
    let skipped = root.join("skipped").join("ignored.mts");
    let mocks = discover(&root, &["skipped".to_string()]);
    assert!(!mocks.contains(&skipped));
}
