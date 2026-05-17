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
