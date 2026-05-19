use super::*;

#[test]
fn rule_enabled_requires_configured_rule() {
    let mut config = crate::config::v2::NoMistakesConfig::default();
    assert!(!rule_enabled(&config, TEST_NO_UNMOCKED_DYNAMIC_IMPORTS));
    config.rules.insert(
        TEST_NO_UNMOCKED_DYNAMIC_IMPORTS.to_string(),
        serde_yaml::from_str("{}").unwrap(),
    );
    assert!(rule_enabled(&config, TEST_NO_UNMOCKED_DYNAMIC_IMPORTS));
}

#[test]
fn rule_enabled_accepts_project_rule_without_top_level_options() {
    let mut config = crate::config::v2::NoMistakesConfig::default();
    config.projects.insert(
        "tests".to_string(),
        crate::config::v2::schema::Project {
            rules: vec![TEST_NO_UNMOCKED_DYNAMIC_IMPORTS.to_string()],
            ..Default::default()
        },
    );
    assert!(rule_enabled(&config, TEST_NO_UNMOCKED_DYNAMIC_IMPORTS));
}

#[test]
fn run_check_returns_empty_when_rule_is_not_enabled() {
    let root = std::path::Path::new("/tmp/no-mistakes-empty-rules");
    let findings = run_check(root, None, None).unwrap();
    assert!(findings.is_empty());
}

#[test]
fn run_check_executes_enabled_rule() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis/test-no-unmocked-dynamic-imports");

    run_check(&root, None, None).unwrap();
}

fn dynamic_import_fixture() -> std::path::PathBuf {
    crate::codebase::ts_resolver::normalize_path(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis/test-no-unmocked-dynamic-imports"),
    )
}

fn dynamic_import_test_facts(
    path: &std::path::Path,
    source: &str,
) -> crate::codebase::check_facts::CheckFileFacts {
    crate::codebase::check_facts::CheckFileFacts {
        source: Some(source.to_string()),
        imports: crate::codebase::dependencies::extract::ImportExtractor::for_typescript()
            .unwrap()
            .extract(source)
            .unwrap(),
        dynamic_imports: Some(
            test_no_unmocked_dynamic_imports::ast::extract(path, source).unwrap(),
        ),
        ..Default::default()
    }
}

#[test]
fn run_check_with_facts_reports_missing_test_facts() {
    let root = dynamic_import_fixture();
    let test = root.join("tests/bad.test.mts");
    let shared = crate::codebase::check_facts::CheckFactMap {
        files: vec![test.clone()],
        ..Default::default()
    };

    let error = run_check_with_facts(&root, None, None, &shared).unwrap_err();

    assert!(error.to_string().contains("missing shared facts"));
}

#[test]
fn run_check_with_facts_reports_missing_source_and_dynamic_facts() {
    let root = dynamic_import_fixture();
    let test = root.join("tests/bad.test.mts");
    let mut shared = crate::codebase::check_facts::CheckFactMap {
        files: vec![test.clone()],
        ..Default::default()
    };
    shared.ts.insert(test.clone(), Default::default());

    let missing_source = run_check_with_facts(&root, None, None, &shared).unwrap_err();
    assert!(format!("{missing_source:#}").contains("missing source facts"));

    shared.ts.insert(
        test,
        crate::codebase::check_facts::CheckFileFacts {
            source: Some("it('x', async () => {})".to_string()),
            ..Default::default()
        },
    );
    let missing_dynamic = run_check_with_facts(&root, None, None, &shared).unwrap_err();
    assert!(format!("{missing_dynamic:#}").contains("missing dynamic import facts"));
}

#[test]
fn run_check_with_facts_skips_disabled_parse_errors() {
    let root = dynamic_import_fixture();
    let test = root.join("tests/disabled.test.mts");
    let source =
        "// guardrails-disable-file test-no-unmocked-dynamic-imports\nexport const Broken =";
    let mut shared = crate::codebase::check_facts::CheckFactMap {
        files: vec![test.clone()],
        ..Default::default()
    };
    shared.ts.insert(
        test,
        crate::codebase::check_facts::CheckFileFacts {
            source: Some(source.to_string()),
            parse_error: Some("bad syntax".to_string()),
            ..Default::default()
        },
    );

    run_check_with_facts(&root, None, None, &shared).unwrap();
}

#[test]
fn run_check_with_facts_executes_valid_shared_facts() {
    let root = dynamic_import_fixture();
    let files = crate::codebase::ts_source::discover_files(&root, &[]);
    let facts = crate::codebase::check_facts::collect_check_facts(
        &root,
        files,
        crate::codebase::check_facts::CheckFactPlan {
            imports: true,
            dynamic_imports: true,
            source: true,
            ..Default::default()
        },
    );

    run_check_with_facts(&root, None, None, &facts).unwrap();
}

#[test]
fn run_check_with_facts_resolves_setup_mocks() {
    let root = dynamic_import_fixture();
    let test = root.join("tests/setup-good.test.mts");
    let setup = root.join("tests/setup-vitest.mts");
    let mut shared = crate::codebase::check_facts::CheckFactMap {
        files: vec![test.clone(), setup.clone()],
        ..Default::default()
    };
    shared.ts.insert(
        test.clone(),
        dynamic_import_test_facts(
            &test,
            "import { expect, test } from 'vitest'\n\
test('setup file mock counts', async () => {\n\
  const mod = await import('@lib/setup-target.mts')\n\
  expect(mod.setupValue).toBe('mocked')\n\
})\n",
        ),
    );
    shared.ts.insert(
        setup.clone(),
        dynamic_import_test_facts(
            &setup,
            "import { vi } from 'vitest'\n\
vi.mock('@lib/setup-target.mts', () => ({ setupValue: 'mocked' }))\n",
        ),
    );

    run_check_with_facts(&root, None, None, &shared).unwrap();
}

#[test]
fn run_check_with_facts_skips_reachable_deps_with_parse_errors() {
    let root = dynamic_import_fixture();
    let test = root.join("tests/bad.test.mts");
    let setup = root.join("tests/setup-vitest.mts");
    // src/unreadable.mts is a directory on disk, so collect_check_facts will
    // store a parse_error for it in CheckFactMap.
    let unreadable = root.join("src/unreadable.mts");
    let files = vec![test.clone(), setup, unreadable];
    let facts = crate::codebase::check_facts::collect_check_facts(
        &root,
        files,
        crate::codebase::check_facts::CheckFactPlan {
            imports: true,
            dynamic_imports: true,
            source: true,
            ..Default::default()
        },
    );
    let source = "import '@lib/unreadable.mts'\n\
test('bad', async () => {\n\
  await import('@lib/setup-target.mts')\n\
})\n";
    let files = facts.files().to_vec();
    let mut shared = crate::codebase::check_facts::CheckFactMap {
        files,
        ts: facts.ts,
        ..Default::default()
    };
    shared
        .ts
        .insert(test.clone(), dynamic_import_test_facts(&test, source));

    // Reachable deps with parse_error in CheckFactMap are silently skipped
    // rather than re-attempted from disk, so the check succeeds.
    run_check_with_facts(&root, None, None, &shared).unwrap();
}

#[test]
fn run_check_with_facts_propagates_reachable_dep_disk_error() {
    // Coverage for with_facts.rs: reachable::check error branch.
    // unreadable.mts is a directory; putting it in shared.files but not shared.ts
    // causes reachable::check to fall back to disk and fail.
    let root = dynamic_import_fixture();
    let test = root.join("tests/bad.test.mts");
    let setup = root.join("tests/setup-vitest.mts");
    let unreadable = root.join("src/unreadable.mts");
    let source = "import '@lib/unreadable.mts'\n\
test('bad', async () => {\n\
  await import('@lib/setup-target.mts')\n\
})\n";
    let setup_source = std::fs::read_to_string(&setup).unwrap();
    let mut shared = crate::codebase::check_facts::CheckFactMap {
        files: vec![test.clone(), setup.clone(), unreadable],
        ..Default::default()
    };
    shared
        .ts
        .insert(test.clone(), dynamic_import_test_facts(&test, source));
    shared.ts.insert(
        setup.clone(),
        dynamic_import_test_facts(&setup, &setup_source),
    );
    let error = run_check_with_facts(&root, None, None, &shared).unwrap_err();
    assert!(error.to_string().contains("failed to read dependency file"));
}

#[test]
fn run_check_with_facts_reports_missing_setup_fact_shapes() {
    let root = dynamic_import_fixture();
    let test = root.join("tests/setup-good.test.mts");
    let setup = root.join("tests/setup-vitest.mts");
    let test_source = "test('setup file mock counts', async () => {\n\
  await import('@lib/setup-target.mts')\n\
})\n";
    let mut shared = crate::codebase::check_facts::CheckFactMap {
        files: vec![test.clone()],
        ..Default::default()
    };
    shared
        .ts
        .insert(test.clone(), dynamic_import_test_facts(&test, test_source));

    let missing = run_check_with_facts(&root, None, None, &shared).unwrap_err();
    assert!(missing.to_string().contains("missing shared facts"));

    shared.files.push(setup.clone());
    shared.ts.insert(
        setup.clone(),
        crate::codebase::check_facts::CheckFileFacts {
            parse_error: Some("bad setup".to_string()),
            ..Default::default()
        },
    );
    let parse_error = run_check_with_facts(&root, None, None, &shared).unwrap_err();
    assert!(parse_error.to_string().contains("bad setup"));

    shared.ts.insert(
        setup,
        crate::codebase::check_facts::CheckFileFacts {
            source: Some("vi.mock('@lib/setup-target.mts')".to_string()),
            ..Default::default()
        },
    );
    let missing_dynamic = run_check_with_facts(&root, None, None, &shared).unwrap_err();
    assert!(missing_dynamic
        .to_string()
        .contains("missing dynamic import facts"));
}

#[test]
fn run_check_with_facts_reports_test_file_parse_error() {
    // with_facts.rs:48 — parse_error bail for the test file itself (without disable comment)
    let root = dynamic_import_fixture();
    let test = root.join("tests/bad.test.mts");
    let mut shared = crate::codebase::check_facts::CheckFactMap {
        files: vec![test.clone()],
        ..Default::default()
    };
    shared.ts.insert(
        test,
        crate::codebase::check_facts::CheckFileFacts {
            source: Some("test('broken', () => {})".to_string()),
            parse_error: Some("syntax error".to_string()),
            ..Default::default()
        },
    );
    let error = run_check_with_facts(&root, None, None, &shared).unwrap_err();
    assert!(format!("{error:#}").contains("syntax error"));
}
