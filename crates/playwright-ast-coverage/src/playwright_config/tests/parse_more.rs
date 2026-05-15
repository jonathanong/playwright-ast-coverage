use crate::playwright_config::parse::parse;
use crate::test_support::fixture_source;
use std::path::Path;

#[test]
fn resolves_identifier_backed_use_object() {
    let source = fixture_source(&["playwright_config", "use-identifier.ts"]);
    let parsed = parse(&source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects[0].test_dir, "./use-identifier-tests");
    assert_eq!(
        parsed.projects[0].base_url.as_deref(),
        Some("http://localhost:6200")
    );
    assert_eq!(parsed.projects[0].test_id_attribute, "data-shared");
}

#[test]
fn cyclic_identifier_configs_fall_back_without_recursing() {
    let source = fixture_source(&["playwright_config", "cyclic-config.ts"]);
    let parsed = parse(&source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects[0].test_dir, ".");
}

#[test]
fn template_literals_use_cooked_text() {
    let source = fixture_source(&["playwright_config", "cooked-template.ts"]);
    let parsed = parse(&source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects[0].test_dir, r#"tests\e2e"#);
    assert_eq!(parsed.projects[0].test_match, vec![r#"**\/*.spec.ts"#]);
}

#[test]
fn parser_handles_advanced_export_shapes() {
    let source = fixture_source(&["playwright_config", "advanced-export-shapes.ts"]);
    let parsed = parse(&source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects[0].test_dir, "./advanced-export-tests");
    assert_eq!(
        parsed.projects[0].test_match,
        vec!["**/*.advanced-export.ts"]
    );

    let source = fixture_source(&["playwright_config", "non-object-binding.ts"]);
    let parsed = parse(&source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects[0].test_dir, ".");
}

#[test]
fn ignores_non_literal_optional_playwright_values() {
    let source = fixture_source(&["playwright_config", "nonliteral-optional-values.ts"]);
    let parsed = parse(&source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects[0].test_dir, "./tests");
    assert_eq!(parsed.projects[0].base_url, None);
    assert_eq!(parsed.projects[0].test_id_attribute, "data-testid");
}

#[test]
fn parse_accepts_spaced_property_and_escaped_string() {
    let source = fixture_source(&["playwright_config", "spaced-property.ts"]);
    let parsed = parse(&source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects[0].test_dir, r#"tests\e2e"#);
}

#[test]
fn parser_rejects_unsupported_required_values() {
    assert!(parse("export default { testDir: 123 }", Path::new("/repo")).is_err());
    assert!(parse("export default { testIgnore: 123 }", Path::new("/repo")).is_err());
    assert!(parse(
        "export default { testMatch: [/.*\\.spec\\.ts/] }",
        Path::new("/repo")
    )
    .is_err());
    assert!(parse("export default { testMatch: 123 }", Path::new("/repo")).is_err());
    assert!(parse(
        "export default { projects: [{ testDir: 123 }] }",
        Path::new("/repo")
    )
    .is_err());
}

#[test]
fn malformed_projects_value_falls_back_to_single_project() {
    let parsed = parse(
        "export default { projects: makeProjects() }",
        Path::new("/repo"),
    )
    .unwrap();
    assert_eq!(parsed.projects.len(), 1);
}

#[test]
fn root_options_ignore_project_values() {
    let source = fixture_source(&["playwright_config", "project-values-only.ts"]);
    let parsed = parse(&source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects[0].test_dir, "./project-tests");
    assert_eq!(parsed.projects[0].test_match, vec!["**/*.project.ts"]);
}

#[test]
fn double_parenthesized_array_is_accepted() {
    let source = "export default { testMatch: ((['**/*.spec.ts'])) };";
    let parsed = parse(source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects[0].test_match, vec!["**/*.spec.ts"]);
}

#[test]
fn unsupported_array_elements_are_rejected() {
    assert!(parse("export default { testMatch: [foo] }", Path::new("/repo")).is_err());
    assert!(parse("export default { testMatch: [] }", Path::new("/repo")).is_err());
}

#[test]
fn parser_handles_ast_edge_shapes() {
    let source = fixture_source(&["playwright_config", "no-default-export.ts"]);
    let parsed = parse(&source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects[0].test_dir, ".");

    let source = fixture_source(&["playwright_config", "non-object-default.ts"]);
    let parsed = parse(&source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects[0].test_dir, ".");

    let source = fixture_source(&["playwright_config", "default-function.ts"]);
    let parsed = parse(&source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects[0].test_dir, ".");

    let source = fixture_source(&["playwright_config", "edge-shapes.ts"]);
    let parsed = parse(&source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects.len(), 1);
    assert_eq!(parsed.projects[0].test_dir, "./project-tests");
    assert_eq!(parsed.projects[0].test_match, vec!["**/*.project.ts"]);
    assert_eq!(parsed.projects[0].test_ignore, vec!["**/skip/**"]);
    assert_eq!(
        parsed.projects[0].base_url.as_deref(),
        Some("http://localhost:3000")
    );
    assert_eq!(parsed.projects[0].test_id_attribute, "data-test");
}
