use super::*;
use oxc_ast_visit::{walk, Visit};
use oxc_span::Span;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/integration-tests")
            .join(name),
    )
}

fn fixture_file(name: &str, file: &str) -> PathBuf {
    fixture(name).join(file)
}

#[test]
fn check_reports_integration_policy_violations() {
    let findings = check(&fixture("basic"), None).unwrap();
    let messages: Vec<_> = findings
        .iter()
        .map(|finding| {
            (
                finding.framework.as_str(),
                finding.suite.as_str(),
                finding.file.as_str(),
                finding.test_name.as_deref(),
                finding.integration.as_deref(),
            )
        })
        .collect();

    assert!(messages.contains(&(
        "vitest",
        "unit",
        "backend/unit.test.mts",
        Some("direct integration in unit suite"),
        Some("openai"),
    )));
    assert!(messages.contains(&(
        "vitest",
        "unit",
        "backend/unit.test.mts",
        Some("helper integration in unit suite"),
        Some("openai"),
    )));
    assert!(messages.contains(&(
        "vitest",
        "openai",
        "integration/openai.test.mts",
        Some("strict suite requires annotation"),
        None,
    )));
    assert!(messages.contains(&(
        "vitest",
        "mixed",
        "mixed/mixed.test.mts",
        Some("wrong integration still fails in non-strict suite"),
        Some("anthropic"),
    )));
    assert!(messages.contains(&(
        "playwright",
        "pw-unit",
        "playwright/unit/unit.spec.ts",
        Some("playwright helper integration in unit suite"),
        Some("openai"),
    )));
    assert!(messages.contains(&(
        "playwright",
        "pw-openai",
        "playwright/openai/openai.spec.ts",
        Some("playwright strict requires integration"),
        None,
    )));
    assert_eq!(findings.len(), 8);
}

#[test]
fn invalid_integration_true_is_rejected() {
    let yaml = "tests:\n  vitest:\n    suites:\n      - name: bad\n        integration: true\n";
    let config: crate::config::v2::schema::NoMistakesConfig = serde_yaml::from_str(yaml).unwrap();
    let err = config::validate_config(&config).unwrap_err();
    assert!(err
        .to_string()
        .contains("integration: true is not supported"));
}

#[test]
fn invalid_empty_integration_suites_is_rejected() {
    let yaml = "tests:\n  vitest:\n    suites:\n      - name: bad\n        integration:\n          suites: []\n";
    let config: crate::config::v2::schema::NoMistakesConfig = serde_yaml::from_str(yaml).unwrap();
    let err = config::validate_config(&config).unwrap_err();
    assert!(err
        .to_string()
        .contains("integration.suites must contain at least one name"));
}

#[test]
fn annotation_requires_one_valid_value() {
    let valid = "const f = /* no-mistakes: integration=openai */ async () => {}";
    let valid_start = valid.find("async").unwrap() as u32;
    assert_eq!(
        calls::integration_annotation_before(valid, Span::new(valid_start, valid_start + 5))
            .as_deref(),
        Some("openai")
    );

    let invalid = "const f = /* no-mistakes: integration=openai,anthropic */ async () => {}";
    let invalid_start = invalid.find("async").unwrap() as u32;
    assert!(calls::integration_annotation_before(
        invalid,
        Span::new(invalid_start, invalid_start + 5)
    )
    .is_none());
}

#[test]
fn coverage_fixture_exercises_parser_and_resolution_variants() {
    let root = fixture("coverage");
    let findings = check(&root, None).unwrap();
    assert!(findings.iter().any(|finding| {
        finding.framework == "vitest"
            && finding.suite == "root-vitest"
            && finding.test_name.as_deref() == Some("uses declared function")
            && finding.integration.as_deref() == Some("openai")
    }));
    assert!(findings.iter().any(|finding| {
        finding.suite == "default-globs"
            && finding.test_name.as_deref() == Some("uses namespace function")
            && finding.integration.as_deref() == Some("openai")
    }));
    assert!(findings
        .iter()
        .all(|finding| finding.suite != "nested-suite"));
}

#[test]
fn invalid_suite_project_and_missing_config_are_rejected() {
    let missing = check(&fixture("missing-config"), None).unwrap_err();
    assert!(missing.to_string().contains("config does not exist"));

    let unknown = check(&fixture("unknown-project"), None).unwrap_err();
    assert!(unknown
        .to_string()
        .contains("vitest suite missing references unknown project missing"));
}

#[test]
fn configured_suites_cover_matching_variants() {
    let root = fixture("coverage");
    let yaml = r#"
tests:
  playwright:
    configs: playwright.projects.ts
    suites:
      - project: inherits
        integration: false
      - name: by-config
        config: playwright.projects.ts
        include: ['custom/**/*.spec.ts']
        integration: false
  vitest:
    configs: vitest.object.mts
    suites:
      - name: default-name
        integration: false
"#;
    let config: crate::config::v2::schema::NoMistakesConfig = serde_yaml::from_str(yaml).unwrap();
    let suites = config::configured_suites(&root, &config).unwrap();
    assert!(suites.iter().any(|suite| suite.name == "inherits"));
    assert!(suites
        .iter()
        .any(|suite| suite.name == "by-config" && suite.include == vec!["custom/**/*.spec.ts"]));
    assert!(suites.iter().any(|suite| suite.name == "default-name"));

    let missing_config = r#"
tests:
  playwright:
    configs: playwright.projects.ts
    suites:
      - config: missing.ts
        integration: false
"#;
    let config: crate::config::v2::schema::NoMistakesConfig =
        serde_yaml::from_str(missing_config).unwrap();
    let err = config::configured_suites(&root, &config).unwrap_err();
    assert!(err.to_string().contains("config missing.ts"));

    assert!(
        project_config::load_projects(&root, types::Framework::Vitest, None)
            .unwrap()
            .is_empty()
    );
    assert!(project_config::resolve_tsconfig(&root)
        .unwrap()
        .base_url
        .is_some());
    assert!(project_config::build_globset(&["[".to_string()]).is_err());
    assert_eq!(
        config::policy_target(&crate::config::v2::schema::TestSuitePolicy::default()),
        "suite"
    );
    assert!(!project_config::load_projects(
        &root,
        types::Framework::Playwright,
        Some(&crate::config::v2::schema::StringOrList::One(
            "playwright.projects.ts".to_string()
        )),
    )
    .unwrap()
    .is_empty());
    assert!(project_config::load_projects(
        &root,
        types::Framework::Playwright,
        Some(&crate::config::v2::schema::StringOrList::One(
            "playwright.invalid.ts".to_string()
        )),
    )
    .is_err());

    let missing_config_and_project = r#"
tests:
  playwright:
    configs: playwright.projects.ts
    suites:
      - config: missing.ts
        project: missing
        integration: false
"#;
    let config: crate::config::v2::schema::NoMistakesConfig =
        serde_yaml::from_str(missing_config_and_project).unwrap();
    let err = config::configured_suites(&root, &config).unwrap_err();
    assert!(err
        .to_string()
        .contains("config missing.ts project missing"));
}

#[test]
fn analyze_files_covers_import_and_function_shapes() {
    let file = fixture_file("coverage", "src/source.test.ts");
    let missing = fixture_file("coverage", "src/does-not-exist.ts");
    let analyses = analysis::analyze_files(&[missing, file.clone()]).unwrap();
    let analysis = analyses.get(&file).unwrap();

    assert!(analysis.imports.contains_key("defaultCall"));
    assert!(analysis.imports.contains_key("renamedCall"));
    assert!(analysis.imports.contains_key("helperNamespace"));
    assert!(analysis.functions.contains_key("declaredIntegration"));
    assert!(analysis.functions.contains_key("arrowIntegration"));
    assert!(analysis.functions.contains_key("functionIntegration"));
    assert!(analysis.functions.contains_key("exportedDeclared"));
    assert!(analysis.functions.contains_key("exportedArrow"));
    assert!(analysis.functions.contains_key("exportedFunction"));
    assert!(analysis
        .tests
        .iter()
        .any(|test| test.name.as_deref() == Some("uses declared function")));
}

#[test]
fn playwright_config_parser_covers_project_defaults() {
    let root = fixture("coverage");
    let path = root.join("playwright.projects.ts");
    let source = std::fs::read_to_string(&path).unwrap();
    let parsed = test_config::playwright::parse_from_path(&source, &path, &root).unwrap();
    let projects = parsed.into_projects(&root, "playwright.projects.ts");

    assert!(projects.iter().any(|project| {
        project.name.as_deref() == Some("absolute")
            && project.include == vec!["/tmp/no-mistakes-absolute-tests/**/*.spec.ts"]
    }));
    assert!(projects.iter().any(|project| {
        project.name.as_deref() == Some("inherits")
            && project
                .exclude
                .iter()
                .any(|glob| glob.ends_with("root-ignore.ts"))
    }));

    let empty_path = root.join("playwright.empty.ts");
    let empty = std::fs::read_to_string(&empty_path).unwrap();
    let parsed = test_config::playwright::parse_from_path(&empty, &empty_path, &root).unwrap();
    assert_eq!(parsed.into_projects(&root, "playwright.empty.ts").len(), 1);

    let parsed =
        test_config::playwright::parse_from_path(&empty, &empty_path, "relative".as_ref()).unwrap();
    assert!(parsed.into_projects(&root, "relative.ts")[0].include[0].starts_with("relative/"));
}

#[test]
fn vitest_config_parser_covers_root_and_nested_projects() {
    let root = fixture("coverage");
    let object_path = root.join("vitest.object.mts");
    let object_source = std::fs::read_to_string(&object_path).unwrap();
    let object_projects =
        test_config::vitest::parse_from_path(&object_source, &object_path, &root, &root).unwrap();
    assert_eq!(object_projects[0].name.as_deref(), Some("root-vitest"));

    let projects_path = root.join("vitest.projects.mts");
    let projects_source = std::fs::read_to_string(&projects_path).unwrap();
    let projects =
        test_config::vitest::parse_from_path(&projects_source, &projects_path, &root, &root)
            .unwrap();
    assert!(projects
        .iter()
        .any(|project| project.name.as_deref() == Some("nested")));
    assert!(projects
        .iter()
        .any(|project| project.name.as_deref() == Some("root")));

    let empty_path = root.join("vitest.empty.mts");
    let empty_source = std::fs::read_to_string(&empty_path).unwrap();
    assert!(
        test_config::vitest::parse_from_path(&empty_source, &empty_path, &root, &root)
            .unwrap()
            .is_empty()
    );

    let defaults_path = root.join("vitest.defaults.mts");
    let defaults_source = std::fs::read_to_string(&defaults_path).unwrap();
    let defaults =
        test_config::vitest::parse_from_path(&defaults_source, &defaults_path, &root, &root)
            .unwrap();
    assert!(defaults[0]
        .include
        .iter()
        .any(|glob| glob.contains("__tests__")));
}

#[test]
fn call_helpers_cover_non_test_and_member_variants() {
    let path = fixture_file("coverage", "src/calls.ts");
    let source = std::fs::read_to_string(&path).unwrap();
    crate::ast::with_program(&path, &source, |program, _| {
        let mut collector = CallAssertions::default();
        collector.visit_program(program);
        assert!(collector.saw_describe_as_non_test);
        assert!(collector.saw_non_string_test);
        assert!(collector.saw_function_callback);
        assert!(collector.saw_imported_member_call);
        assert!(collector.saw_non_callback_argument);
    })
    .unwrap();
}

#[test]
fn config_parsers_reject_invalid_literals() {
    let root = fixture("coverage");
    let pw_path = root.join("playwright.invalid.ts");
    let pw_source = std::fs::read_to_string(&pw_path).unwrap();
    let pw_err = match test_config::playwright::parse_from_path(&pw_source, &pw_path, &root) {
        Ok(_) => panic!("expected invalid Playwright config to fail"),
        Err(err) => err,
    };
    assert!(pw_err.to_string().contains("expected string literal"));

    let vitest_path = root.join("vitest.invalid.mts");
    let vitest_source = std::fs::read_to_string(&vitest_path).unwrap();
    let vitest_err =
        test_config::vitest::parse_from_path(&vitest_source, &vitest_path, &root, &root)
            .unwrap_err();
    assert!(vitest_err
        .to_string()
        .contains("expected string literal array entries"));
}

#[test]
fn shared_config_helpers_cover_ast_edge_shapes() {
    let path = fixture_file("coverage", "parser-helpers.ts");
    let source = std::fs::read_to_string(&path).unwrap();
    crate::ast::with_program(&path, &source, |program, source| {
        let bindings = test_config::shared::top_level_object_bindings(program);
        assert!(bindings.contains_key("nested"));
        assert!(!bindings.contains_key("noInit"));
        assert!(!bindings.contains_key("destructured"));

        let object = test_config::shared::default_export_object(program, &bindings, true).unwrap();
        assert_eq!(
            test_config::shared::property_expression(object, "name")
                .and_then(|expr| test_config::shared::optional_string(expr, source))
                .as_deref(),
            Some("nested")
        );

        let fixture_object = test_config::shared::property_object(object, "missing", &bindings);
        assert!(fixture_object.is_none());
        let oxc_ast::ast::Expression::ObjectExpression(object) = bindings.get("object").unwrap()
        else {
            panic!("expected object binding");
        };
        assert_eq!(
            test_config::shared::property_expression(object, "name")
                .map(|expr| test_config::shared::required_string(expr, source, "name").unwrap())
                .as_deref(),
            Some("literal")
        );
        assert!(test_config::shared::property_expression(object, "computed").is_none());
        assert!(test_config::shared::property_expression(object, "quoted").is_some());
        assert_eq!(test_config::shared::project_objects(object).len(), 1);

        let list = test_config::shared::property_expression(object, "list").unwrap();
        assert_eq!(
            test_config::shared::required_string_or_array(list, source, "list").unwrap(),
            vec!["one".to_string(), "two".to_string()]
        );
        let wrapped_list = test_config::shared::property_expression(object, "wrappedList").unwrap();
        assert_eq!(
            test_config::shared::required_string_or_array(wrapped_list, source, "wrappedList")
                .unwrap(),
            vec!["three".to_string()]
        );
        let non_array = test_config::shared::property_expression(object, "nonArray").unwrap();
        assert!(
            test_config::shared::required_string_or_array(non_array, source, "nonArray").is_err()
        );
        let bad_list = test_config::shared::property_expression(object, "badList").unwrap();
        assert!(
            test_config::shared::required_string_or_array(bad_list, source, "badList").is_err()
        );
    })
    .unwrap();

    let path = fixture_file("coverage", "parser-edge.ts");
    let source = std::fs::read_to_string(&path).unwrap();
    crate::ast::with_program(&path, &source, |program, _| {
        let bindings = test_config::shared::top_level_object_bindings(program);
        assert!(test_config::shared::default_export_object(program, &bindings, true).is_none());
        let oxc_ast::ast::Expression::ObjectExpression(object) = bindings.get("object").unwrap()
        else {
            panic!("expected object binding");
        };
        assert!(test_config::shared::property_expression(object, "quoted").is_some());
        assert!(test_config::shared::property_object(object, "cyclic", &bindings).is_none());
    })
    .unwrap();

    let path = fixture_file("coverage", "parser-cycle.ts");
    let source = std::fs::read_to_string(&path).unwrap();
    crate::ast::with_program(&path, &source, |program, _| {
        let bindings = test_config::shared::top_level_object_bindings(program);
        assert!(test_config::shared::default_export_object(program, &bindings, true).is_none());
    })
    .unwrap();

    let path = fixture_file("coverage", "playwright.call-invalid.ts");
    let source = std::fs::read_to_string(&path).unwrap();
    crate::ast::with_program(&path, &source, |program, _| {
        let bindings = test_config::shared::top_level_object_bindings(program);
        assert!(test_config::shared::default_export_object(program, &bindings, true).is_none());
    })
    .unwrap();
}

#[derive(Default)]
struct CallAssertions {
    saw_describe_as_non_test: bool,
    saw_non_string_test: bool,
    saw_function_callback: bool,
    saw_imported_member_call: bool,
    saw_non_callback_argument: bool,
}

impl<'a> Visit<'a> for CallAssertions {
    fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
        let path = crate::ast::expression_path(&call.callee);
        if path
            .as_ref()
            .is_some_and(|path| path == &["test", "describe"])
        {
            self.saw_describe_as_non_test = calls::test_name(call).is_none();
        }
        if path.as_ref().is_some_and(|path| path == &["test"]) && calls::test_name(call).is_none() {
            self.saw_non_string_test = true;
            self.saw_non_callback_argument = calls::callback_argument(call).is_none();
            assert!(calls::collect_calls(call.arguments.first().unwrap()).is_empty());
        }
        if calls::test_name(call).as_deref() == Some("function callback") {
            let (argument, _) = calls::callback_argument(call).unwrap();
            let collected = calls::collect_calls(argument);
            self.saw_function_callback = true;
            self.saw_imported_member_call = collected.iter().any(
                |target| matches!(target, types::CallTarget::Imported { local } if local == "foo"),
            );
        }
        walk::walk_call_expression(self, call);
    }
}
