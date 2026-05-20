use super::*;
use crate::config::v2::schema::{Project, RuleDef};

mod rule_targets;

fn apply_rule_to_projects(config: &mut NoMistakesConfig, projects: &[&str]) {
    config.rules.push(RuleDef {
        rule: super::super::RULE_ID.to_string(),
        projects: projects
            .iter()
            .map(|project| (*project).to_string())
            .collect(),
        ..Default::default()
    });
}

fn setup_files(root: &Path, config: &NoMistakesConfig) -> Result<Vec<PathBuf>> {
    let cfg_files = config_files(root, config)
        .into_iter()
        .map(|config| config.path)
        .collect::<Vec<_>>();
    setup_files_from_configs(root, cfg_files)
}

fn setup_files_for_test(
    root: &Path,
    config: &NoMistakesConfig,
    rel_path: String,
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for config_file in config_files(root, config) {
        let source = std::fs::read_to_string(&config_file.path)?;
        let base = config_file.path.parent().unwrap_or(root);
        let includes = normalize_matcher_patterns(root, base, config_file.includes(&source));
        let excludes = normalize_matcher_patterns(
            root,
            base,
            extract_test_property_strings(&source, "exclude"),
        );
        let filter = TestFilter {
            include: build_globset(&includes)?,
            include_regex: build_regexes(&extract_test_regexes(&source))?,
            exclude: build_globset(&excludes)?,
        };
        if filter.is_match(&rel_path) {
            files.extend(setup_files_from_configs(root, vec![config_file.path])?);
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

#[test]
fn extracts_setup_files_and_include_exclude_strings() {
    let source = "export default { coverage: { exclude: ['ignored.mts'] }, test: { include: ['a.test.mts'], exclude: ['b.test.mts'], setupFiles: ['./setup.mts'] } }";
    assert_eq!(
        extract_test_property_strings(source, "setupFiles"),
        vec!["./setup.mts"]
    );
    assert_eq!(
        extract_test_property_strings(source, "include"),
        vec!["a.test.mts"]
    );
    assert_eq!(
        extract_test_property_strings(source, "exclude"),
        vec!["b.test.mts"]
    );
    assert!(extract_test_property_strings(source, "exclude")
        .iter()
        .all(|value| value != "ignored.mts"));
}

#[test]
fn default_filter_matches_vitest_and_jest_test_files() {
    let config = NoMistakesConfig::default();
    let filter = test_filter(Path::new("."), &config).unwrap();
    assert!(filter.is_match("src/a.test.mts"));
    assert!(filter.is_match("src/a.spec.ts"));
    assert!(filter.is_match("src/__tests__/a.js"));
    assert!(!filter.is_match("src/a.mts"));
}

#[test]
fn project_include_restricts_default_test_globs() {
    let mut config = NoMistakesConfig::default();
    config.projects.insert(
        "storybook".to_string(),
        Project {
            root: Some("web/storybook".to_string()),
            include: vec!["**/*.test.tsx".to_string()],
            ..Default::default()
        },
    );
    config.projects.insert(
        "root-tests".to_string(),
        Project {
            include: vec!["tests/**/*.test.ts".to_string()],
            ..Default::default()
        },
    );
    config.projects.insert(
        "other".to_string(),
        Project {
            include: vec!["other/**/*.test.ts".to_string()],
            ..Default::default()
        },
    );
    apply_rule_to_projects(&mut config, &["storybook", "root-tests"]);
    let filter = test_filter(Path::new("."), &config).unwrap();
    assert!(filter.is_match("web/storybook/__tests__/a.test.tsx"));
    assert!(filter.is_match("tests/a.test.ts"));
    assert!(!filter.is_match("web/components/a.test.tsx"));
    assert!(!filter.is_match("other/a.test.ts"));
}

#[test]
fn project_include_does_not_widen_to_config_test_globs() {
    let root = crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/test-no-unmocked-dynamic-imports"),
    );
    let mut config = NoMistakesConfig::default();
    config.projects.insert(
        "focused".to_string(),
        Project {
            root: Some("tests".to_string()),
            include: vec!["good.test.mts".to_string()],
            ..Default::default()
        },
    );
    apply_rule_to_projects(&mut config, &["focused"]);
    config.tests.vitest.configs = Some(crate::config::v2::schema::StringOrList::One(
        "vitest.config.mts".to_string(),
    ));

    let filter = test_filter(&root, &config).unwrap();
    assert!(filter.is_match("tests/good.test.mts"));
    assert!(!filter.is_match("tests/bad.test.mts"));
}

#[test]
fn scoped_glob_leaves_root_project_includes_unprefixed() {
    let mut config = NoMistakesConfig::default();
    config.projects.insert(
        "root-tests".to_string(),
        Project {
            root: Some(".".to_string()),
            include: vec!["tests/**/*.test.ts".to_string()],
            ..Default::default()
        },
    );
    config.projects.insert(
        "storybook".to_string(),
        Project {
            root: Some("web/storybook".to_string()),
            include: vec!["./**/*.test.tsx".to_string()],
            ..Default::default()
        },
    );
    apply_rule_to_projects(&mut config, &["root-tests", "storybook"]);
    let filter = test_filter(Path::new("."), &config).unwrap();
    assert!(filter.is_match("tests/example.test.ts"));
    assert!(filter.is_match("web/storybook/example.test.tsx"));
    assert!(!filter.is_match("web/storybook/example.test.ts"));
}

#[test]
fn setup_files_resolves_config_relative_existing_files() {
    let root = crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/test-no-unmocked-dynamic-imports"),
    );
    let config = crate::config::v2::load_v2_config(&root, None).unwrap();
    let files = setup_files(&root, &config).unwrap();
    assert!(files
        .iter()
        .any(|path| path.ends_with("tests/setup-vitest.mts")));
    assert!(files
        .iter()
        .any(|path| path.ends_with("tests/setup-jest.mts")));
    assert!(files
        .iter()
        .any(|path| path.ends_with("tests/setup-jest-after-env.mts")));
}

#[test]
fn setup_files_for_test_uses_default_globs_when_config_has_no_matcher() {
    let root = crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/test-no-unmocked-dynamic-imports"),
    );
    let mut config = NoMistakesConfig::default();
    config.tests.jest.configs = Some(crate::config::v2::schema::StringOrList::One(
        "jest.no-match.config.cjs".to_string(),
    ));
    let files = setup_files_for_test(&root, &config, "tests/good.test.mts".to_string()).unwrap();
    assert!(files
        .iter()
        .any(|path| path.ends_with("tests/setup-jest.mts")));
}

#[test]
fn setup_files_resolves_jest_root_dir_entries() {
    let root = crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/test-no-unmocked-dynamic-imports"),
    );
    let mut config = NoMistakesConfig::default();
    config.tests.jest.configs = Some(crate::config::v2::schema::StringOrList::One(
        "jest.root-dir.config.cjs".to_string(),
    ));
    let files = setup_files_for_test(&root, &config, "tests/good.test.mts".to_string()).unwrap();
    assert!(files
        .iter()
        .any(|path| path.ends_with("tests/setup-jest.mts")));
}

#[test]
fn setup_files_resolves_jest_root_dir_from_config_directory() {
    let root = crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/test-no-unmocked-dynamic-imports"),
    );
    let mut config = NoMistakesConfig::default();
    config.tests.jest.configs = Some(crate::config::v2::schema::StringOrList::One(
        "packages/nested/jest.config.cjs".to_string(),
    ));
    let files = setup_files_for_test(
        &root,
        &config,
        "packages/nested/example.test.mts".to_string(),
    )
    .unwrap();
    assert!(files
        .iter()
        .any(|path| path.ends_with("packages/nested/setup-jest.mts")));
}

#[test]
fn resolve_setup_file_accepts_exact_root_dir_token() {
    let base = Path::new("/repo/config");
    assert_eq!(resolve_setup_file(base, "<rootDir>"), base);
}

#[test]
fn matcher_patterns_are_normalized_to_repo_relative_paths() {
    let root = Path::new("/repo");
    let base = Path::new("/repo/packages/app");
    assert_eq!(
        normalize_matcher_patterns(
            root,
            base,
            vec![
                "./tests/**/*.test.ts".to_string(),
                "<rootDir>/src/**/*.spec.ts".to_string(),
                "<rootDir>".to_string(),
                "**/*.test.ts".to_string(),
            ],
        ),
        vec![
            "packages/app/tests/**/*.test.ts",
            "packages/app/src/**/*.spec.ts",
            "packages/app",
            "**/*.test.ts",
        ]
    );
}

#[test]
fn explicit_config_files_skip_default_discovery() {
    let root = crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/test-no-unmocked-dynamic-imports"),
    );
    let mut config = NoMistakesConfig::default();
    config.tests.vitest.configs = Some(crate::config::v2::schema::StringOrList::One(
        "jest.config.mjs".to_string(),
    ));
    let files = config_files(&root, &config);
    assert_eq!(files.len(), 1);
    assert!(files[0].path.ends_with("jest.config.mjs"));
}

#[test]
fn invalid_config_globs_do_not_block_default_discovery() {
    let root = crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/test-no-unmocked-dynamic-imports"),
    );
    let mut config = NoMistakesConfig::default();
    config.tests.jest.configs = Some(crate::config::v2::schema::StringOrList::One(
        "[".to_string(),
    ));
    let files = config_files(&root, &config);
    assert!(files
        .iter()
        .any(|file| file.path.ends_with("jest.config.mjs")));
}

#[test]
fn default_config_discovery_normalizes_existing_files() {
    let root = crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/test-no-unmocked-dynamic-imports"),
    );
    let files = config_files(&root, &NoMistakesConfig::default());
    assert!(files
        .iter()
        .any(|file| file.path.ends_with("vitest.config.mts")));
    assert!(files
        .iter()
        .any(|file| file.path.ends_with("jest.config.mjs")));
    assert!(files
        .iter()
        .any(|file| file.path.ends_with("jest.config.cjs")));
}

#[test]
fn configured_config_globs_expand_existing_files() {
    let root = crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/test-no-unmocked-dynamic-imports"),
    );
    let mut config = NoMistakesConfig::default();
    config.tests.jest.configs = Some(crate::config::v2::schema::StringOrList::One(
        "jest.config.*".to_string(),
    ));
    let files = config_files(&root, &config);
    assert!(files
        .iter()
        .any(|file| file.path.ends_with("jest.config.mjs")));
    assert!(files
        .iter()
        .any(|file| file.path.ends_with("jest.config.cjs")));
}

#[test]
fn jest_test_regex_is_used_as_include_pattern() {
    let source = "module.exports = { testRegex: ['.*\\\\.spec\\\\.mts$'] }";
    assert_eq!(extract_test_regexes(source), vec![r#".*\\.spec\\.mts$"#]);
}

#[test]
fn jest_test_regex_literal_is_used_as_include_pattern() {
    let source = r#"module.exports = { testRegex: /.*\.test\.mts$/ }"#;
    assert_eq!(extract_test_regexes(source), vec![r#".*\.test\.mts$"#]);
}

#[test]
fn jest_test_regex_literal_accepts_escaped_slashes_and_arrays() {
    let source = r#"module.exports = {
        testRegex: [/stories\/.*\.test\.tsx$/i, /__tests__[/\\].*\.mts$/],
    }"#;
    assert_eq!(
        extract_test_regexes(source),
        vec![r#"stories\/.*\.test\.tsx$"#, r#"__tests__[/\\].*\.mts$"#]
    );
}

#[test]
fn malformed_jest_test_regex_literals_are_ignored() {
    assert!(extract_test_regexes("module.exports = { testRegex: /unterminated").is_empty());
    assert!(extract_test_regexes("module.exports = { testRegex: [/unterminated]").is_empty());
}

#[test]
fn jest_test_match_accepts_bracketed_globs_inside_arrays() {
    let source = r#"module.exports = {
        testMatch: ["**/?(*.)+(spec|test).[jt]s?(x)", "**/__tests__/**/*.mts"],
    }"#;
    assert_eq!(
        extract_property_strings(source, "testMatch"),
        vec!["**/?(*.)+(spec|test).[jt]s?(x)", "**/__tests__/**/*.mts"]
    );
}

#[test]
fn property_string_parser_handles_single_and_malformed_strings() {
    assert_eq!(
        extract_property_strings(
            "module.exports = { testMatch: '**/*.test.ts' }",
            "testMatch"
        ),
        vec!["**/*.test.ts"]
    );
    assert!(
        extract_property_strings("module.exports = { testMatch: \"unterminated", "testMatch")
            .is_empty()
    );
    assert!(extract_property_strings(
        "module.exports = { testMatch: [\"unterminated] }",
        "testMatch"
    )
    .is_empty());
}

#[test]
fn jest_test_regex_does_not_fall_back_to_default_globs() {
    let root = crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/test-no-unmocked-dynamic-imports"),
    );
    let mut config = NoMistakesConfig::default();
    config.tests.jest.configs = Some(crate::config::v2::schema::StringOrList::One(
        "jest.regex.config.cjs".to_string(),
    ));
    let files = config_files(&root, &config);
    let source = std::fs::read_to_string(&files[0].path).unwrap();
    assert!(files[0].includes(&source).is_empty());
}

#[test]
fn vitest_config_without_include_uses_default_globs() {
    let config_file = discovery::ConfigFile {
        path: PathBuf::from("vitest.config.mts"),
        runner: discovery::Runner::Vitest,
    };
    assert!(config_file
        .includes("export default { test: { setupFiles: ['./setup.mts'] } }")
        .iter()
        .any(|pattern| pattern.contains("test")));
}

#[test]
fn test_filter_matches_jest_regex_includes() {
    let root = crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/test-no-unmocked-dynamic-imports"),
    );
    let mut config = NoMistakesConfig::default();
    config.tests.jest.configs = Some(crate::config::v2::schema::StringOrList::One(
        "jest.regex.config.cjs".to_string(),
    ));
    let filter = test_filter(&root, &config).unwrap();
    assert!(filter.is_match("tests/example.regex-test.mts"));
    assert!(!filter.is_match("tests/example.regex-test.ts"));
    assert!(!filter.is_match("tests/plain.test.ts"));
}

#[test]
fn invalid_regex_patterns_return_error() {
    assert!(build_regexes(&["(".to_string()]).is_err());
}

#[test]
fn test_block_parser_ignores_braces_inside_quoted_strings() {
    let source = r#"export default {
        test: {
            nested: { label: "inner" },
            include: ["tests/quoted.test.mts"],
            exclude: ['tests/ignored.test.mts'],
            name: 'escaped \\ backslash',
            title: `template } brace`,
        },
        include: ['outside.test.mts'],
    }"#;
    assert_eq!(
        extract_test_property_strings(source, "include"),
        vec!["tests/quoted.test.mts"]
    );
    assert_eq!(
        extract_test_property_strings(source, "exclude"),
        vec!["tests/ignored.test.mts"]
    );
}

#[test]
fn malformed_test_block_returns_no_properties() {
    let source = "export default { test: { include: ['a.test.mts']";
    assert!(extract_test_property_strings(source, "include").is_empty());
}
