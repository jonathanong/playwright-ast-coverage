use super::*;

#[test]
fn configured_suite_excludes_override_includes() {
    let config: NoMistakesConfig = serde_yaml::from_str(
        r#"
tests:
  vitest:
    suites:
      - name: backend
        include:
          - backend/**/*.test.mts
        exclude:
          - backend/**/*.mock.test.mts
"#,
    )
    .unwrap();
    let filter = TestFileFilter::new(Path::new("."), &config);

    assert!(filter.is_match_rel("backend/api/users.test.mts"));
    assert!(!filter.is_match_rel("backend/api/users.mock.test.mts"));
}
