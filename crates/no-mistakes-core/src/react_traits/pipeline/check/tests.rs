use super::*;

#[test]
fn run_check_with_facts_skips_when_assert_no_fetch_is_disabled() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/react-traits-components/nested");
    let facts = crate::codebase::check_facts::CheckFactMap::default();

    let findings = run_check_with_facts(
        &root,
        None,
        &["app/components/Child.tsx".to_string()],
        false,
        &facts,
    )
    .unwrap();

    assert!(findings.is_empty());
}
