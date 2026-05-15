use crate::playwright_config::parse::{parse, parse_from_path};
use crate::test_support::fixture_source;
use std::path::Path;

#[test]
fn parses_test_dir_and_match() {
    let source = fixture_source(&["ast-snippets", "playwright_config", "test-dir-and-match.ts"]);
    let parsed = parse(&source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.name, None);
    assert_eq!(parsed.projects[0].test_dir, "./tests/e2e");
    assert_eq!(parsed.projects[0].test_match, vec!["**/*.spec.ts"]);
}

#[test]
fn parser_handles_parenthesized_expressions() {
    let source = "export default ({ testDir: 'parenthesized' });";
    let parsed = parse(source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects[0].test_dir, "parenthesized");
}

#[test]
fn property_key_name_handles_edge_cases() {
    let source = "export default { ['computed']: 'value' };";
    let parsed = parse(source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects[0].test_dir, "."); // default if not found
}

#[test]
fn array_element_object_handles_non_objects() {
    let source = "export default { projects: [1] };";
    let parsed = parse(source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects.len(), 1);
}

#[test]
fn property_key_name_handles_all_cases() {
    let source = "export default { testDir: 'v1', 'testMatch': 'v2', [1+1]: 'v3' };";
    let parsed = parse(source, Path::new("/repo")).unwrap();
    assert!(!parsed.projects.is_empty());
}

#[test]
fn parser_handles_advanced_assignment_targets() {
    let source = "export default { use: { baseURL: (x = 'y') } };";
    let parsed = parse(source, Path::new("/repo")).unwrap();
    assert!(!parsed.projects.is_empty());
}

#[test]
fn parses_projects_with_inheritance() {
    let source = fixture_source(&["ast-snippets", "playwright_config", "projects-with-inheritance.ts"]);
    let parsed = parse(&source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects.len(), 2);
    assert_eq!(parsed.projects[0].test_dir, "./tests");
    assert_eq!(parsed.projects[0].test_ignore, vec!["**/skip/**"]);
    assert_eq!(
        parsed.projects[0].base_url.as_deref(),
        Some("http://localhost:3000")
    );
    assert_eq!(parsed.projects[0].test_id_attribute, "data-pw");
    assert_eq!(parsed.projects[1].test_dir, "./e2e");
    assert_eq!(parsed.projects[1].test_match, vec!["**/*.pw.ts"]);
    assert_eq!(parsed.projects[1].test_id_attribute, "data-test");
    assert_eq!(parsed.test_id_attributes(), vec!["data-pw", "data-test"]);
}

#[test]
fn parses_top_level_base_url_and_string_ignore() {
    let source = fixture_source(&["ast-snippets", "playwright_config", "top-level-base-url.ts"]);
    let parsed = parse(&source, Path::new("/repo")).unwrap();
    assert_eq!(
        parsed.projects[0].base_url.as_deref(),
        Some("http://localhost:5173")
    );
    assert_eq!(parsed.projects[0].test_ignore, vec!["**/skip/**"]);
    assert_eq!(parsed.projects[0].test_id_attribute, "data-test-id");
}

#[test]
fn parses_default_export_identifier() {
    let source = fixture_source(&["ast-snippets", "playwright_config", "default-identifier.ts"]);
    let parsed = parse(&source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects[0].test_dir, "./identifier-tests");
    assert_eq!(parsed.projects[0].test_match, vec!["**/*.identifier.ts"]);
    assert_eq!(
        parsed.projects[0].base_url.as_deref(),
        Some("http://localhost:4100")
    );
    assert_eq!(parsed.projects[0].test_id_attribute, "data-identifier");
}

#[test]
fn parses_define_config_identifier_argument() {
    let source = fixture_source(&["ast-snippets", "playwright_config", "define-config-identifier.ts"]);
    let parsed = parse(&source, Path::new("/repo")).unwrap();
    assert_eq!(parsed.projects[0].test_dir, "./define-config-tests");
    assert_eq!(parsed.projects[0].test_match, vec!["**/*.define-config.ts"]);
}

#[test]
fn parses_commonjs_config_exports() {
    let source = fixture_source(&["ast-snippets", "playwright_config", "commonjs-object.cjs"]);
    let parsed = parse_from_path(
        &source,
        Path::new("playwright.config.cjs"),
        Path::new("/repo"),
    )
    .unwrap();
    assert_eq!(parsed.projects[0].test_dir, "./commonjs-object-tests");
    assert_eq!(
        parsed.projects[0].test_match,
        vec!["**/*.commonjs-object.js"]
    );

    // Coverage for assignment_target_path error
    let source = "(1).exports = {}";
    let parsed = parse_from_path(
        source,
        Path::new("playwright.config.cjs"),
        Path::new("/repo"),
    )
    .unwrap();
    assert_eq!(parsed.projects[0].test_dir, ".");
}

#[test]
fn parses_commonjs_define_config_exports() {
    let source = fixture_source(&["ast-snippets", "playwright_config", "commonjs-define-config.cjs"]);
    let parsed = parse_from_path(
        &source,
        Path::new("playwright.config.cjs"),
        Path::new("/repo"),
    )
    .unwrap();
    assert_eq!(
        parsed.projects[0].test_dir,
        "./commonjs-define-config-tests"
    );
    assert_eq!(
        parsed.projects[0].test_match,
        vec!["**/*.commonjs-define-config.js"]
    );
    assert_eq!(
        parsed.projects[0].base_url.as_deref(),
        Some("http://localhost:5100")
    );
}
