use super::*;

#[test]
fn configured_suite_excludes_override_includes() {
    let config = load_config_fixture("suite-exclude");
    let filter = TestFileFilter::new(Path::new("."), &config);

    assert!(filter.is_match_rel("backend/api/users.test.mts"));
    assert!(!filter.is_match_rel("backend/api/users.mock.test.mts"));
    assert!(filter.is_match_rel("integration/api/users.mock.test.mts"));
}

fn load_config_fixture(name: &str) -> NoMistakesConfig {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis/test-filter")
        .join(format!("{name}.yml"));
    let yaml = std::fs::read_to_string(path).unwrap();
    serde_yaml::from_str(&yaml).unwrap()
}
