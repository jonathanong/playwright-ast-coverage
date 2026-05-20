use std::path::Path;

use super::discover::{find_config_root, load_v2_config};
use super::schema::{NoMistakesConfig, ProjectType, RuleDef, StringOrList};
use super::view::ConfigView;

fn fixture(sub: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/config-v2")
        .join(sub)
}

// ── discovery ─────────────────────────────────────────────────────────────────

#[test]
fn empty_config_returns_default() {
    let cfg = load_v2_config(&fixture("empty"), None).unwrap();
    assert_eq!(cfg, NoMistakesConfig::default());
}

#[test]
fn missing_dir_returns_default() {
    let cfg = load_v2_config(Path::new("/tmp/no-mistakes-nonexistent-xyz"), None).unwrap();
    assert_eq!(cfg, NoMistakesConfig::default());
}

#[test]
fn explicit_config_path_overrides_discovery() {
    let dir = fixture("multi-project");
    let explicit = dir.join(".no-mistakes.yml");
    let cfg = load_v2_config(&dir, Some(&explicit)).unwrap();
    assert!(cfg.projects.contains_key("web"));
}

#[test]
fn explicit_legacy_guardrails_path_parsed() {
    let dir = fixture("legacy-guardrails");
    let explicit = dir.join(".guardrailsrc.yml");
    let cfg = load_v2_config(&dir, Some(&explicit)).unwrap();
    assert!(cfg.projects.contains_key("backend"));
}

#[test]
fn explicit_nonexistent_config_errors() {
    let dir = fixture("basic");
    let err = load_v2_config(&dir, Some(Path::new("nonexistent.yml")))
        .err()
        .unwrap();
    assert!(err.to_string().contains("does not exist"));
}

// ── v2 format ─────────────────────────────────────────────────────────────────

#[test]
fn basic_v2_config_parsed() {
    let cfg = load_v2_config(&fixture("basic"), None).unwrap();
    let backend = &cfg.projects["backend"];
    assert_eq!(backend.type_, Some(ProjectType::Server));
    assert_eq!(backend.root.as_deref(), Some("backend"));
    assert!(cfg.rule_configured("http-route-static-paths"));
}

#[test]
fn multi_project_config_parsed() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    assert_eq!(cfg.projects["web"].type_, Some(ProjectType::Nextjs));
    let queues = &cfg.projects["backend"].queues;
    assert_eq!(queues.enqueues, vec!["backend/queues/**"]);
    assert_eq!(queues.workers, vec!["backend/workers/**"]);
    assert_eq!(
        cfg.filesystem.skip_directories,
        vec![".next", "node_modules"]
    );
    let pw = &cfg.tests.playwright;
    assert!(matches!(&pw.configs, Some(StringOrList::One(s)) if s == "playwright.config.ts"));
    assert!(!pw.selectors.html_ids);
    assert_eq!(pw.selectors.test_ids, vec!["data-testid", "data-pw"]);
    assert_eq!(pw.selectors.component_test_ids["dataPw"], "data-pw");
    assert_eq!(pw.selector_roots, vec!["web/app", "web/components"]);
}

#[test]
fn storybook_config_parsed() {
    let cfg = load_v2_config(&fixture("with-storybook"), None).unwrap();
    assert!(matches!(
        &cfg.tests.playwright.configs,
        Some(StringOrList::Many(v)) if v.len() == 2
    ));
    assert!(cfg.tests.storybook.configs.is_some());
    assert!(cfg.tests.vitest.configs.is_some());
}

// ── legacy conversions ────────────────────────────────────────────────────────

#[test]
fn legacy_playwright_converted() {
    let cfg = load_v2_config(&fixture("legacy-playwright"), None).unwrap();
    assert_eq!(cfg.projects["web"].type_, Some(ProjectType::Nextjs));
    assert_eq!(cfg.projects["web"].root.as_deref(), Some("web/app"));
    let pw = &cfg.tests.playwright;
    assert!(matches!(&pw.configs, Some(StringOrList::One(s)) if s == "playwright.config.mts"));
    assert!(pw.selectors.test_ids.contains(&"data-pw".to_string()));
    assert_eq!(pw.selectors.component_test_ids["dataPw"], "data-pw");
    assert_eq!(pw.selector_roots, vec!["web/app", "web/components"]);
}

#[test]
fn legacy_guardrails_converted() {
    let cfg = load_v2_config(&fixture("legacy-guardrails"), None).unwrap();
    assert_eq!(cfg.projects["backend"].root.as_deref(), Some("backend"));
    assert_eq!(
        cfg.filesystem.skip_directories,
        vec![".next", "node_modules"]
    );
    assert!(cfg.rule_configured("http-route-static-paths"));
}

#[test]
fn legacy_react_traits_converted() {
    let cfg = load_v2_config(&fixture("legacy-react-traits"), None).unwrap();
    assert!(cfg.projects.contains_key("web"));
    assert_eq!(cfg.projects["web"].type_, Some(ProjectType::Nextjs));
    assert_eq!(cfg.projects["web"].root.as_deref(), Some("src/app"));
}

#[test]
fn legacy_next_to_fetch_converted() {
    let cfg = load_v2_config(&fixture("legacy-next-to-fetch"), None).unwrap();
    assert!(cfg.projects.contains_key("web"));
    assert_eq!(cfg.projects["web"].root.as_deref(), Some("app"));
}

// ── schema ────────────────────────────────────────────────────────────────────

#[test]
fn string_or_list_values_single() {
    let s = StringOrList::One("foo".to_string());
    assert_eq!(s.values(), vec!["foo"]);
}

#[test]
fn string_or_list_values_many() {
    let s = StringOrList::Many(vec!["a".to_string(), "b".to_string()]);
    assert_eq!(s.values(), vec!["a", "b"]);
}

#[test]
fn rule_def_enabled_defaults_to_true() {
    let yaml = "{}";
    let def: RuleDef = serde_yaml::from_str(yaml).unwrap();
    assert!(def.enabled);
}

#[test]
fn rule_def_options_deserialized() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    let rule = cfg.rule_applications("http-route-static-paths")[0];
    assert_eq!(
        rule.message.as_deref(),
        Some("Route paths must be static literals")
    );
    assert!(rule.enabled);

    #[derive(serde::Deserialize, Default)]
    #[serde(rename_all = "camelCase")]
    struct Opts {
        backend_pattern: String,
    }
    let opts: Opts = rule.rule_options();
    assert_eq!(opts.backend_pattern, "backend/api/**");
}

#[test]
fn rule_def_options_returns_default_on_bad_type() {
    let rule = RuleDef::default();

    #[derive(serde::Deserialize, Default, PartialEq, Debug)]
    struct Opts {
        foo: String,
    }
    let opts: Opts = rule.rule_options();
    assert_eq!(opts, Opts::default());
}

// ── ConfigView ────────────────────────────────────────────────────────────────

#[test]
fn config_view_projects_of_type_filter() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    let view = ConfigView::new(&cfg);
    let nextjs = view.projects_of_type(Some(&ProjectType::Nextjs));
    assert_eq!(nextjs.len(), 1);
    assert_eq!(nextjs[0].0, "web");
    let all = view.projects_of_type(None);
    assert_eq!(all.len(), 2);
}

#[test]
fn config_view_nextjs_root() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    let view = ConfigView::new(&cfg);
    assert_eq!(view.nextjs_root(), "web");
}

#[test]
fn config_view_nextjs_root_default() {
    let cfg = NoMistakesConfig::default();
    let view = ConfigView::new(&cfg);
    assert_eq!(view.nextjs_root(), "app");
}

#[test]
fn config_view_playwright_configs() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    let view = ConfigView::new(&cfg);
    let configs = view.playwright_configs().unwrap();
    assert_eq!(configs, vec!["playwright.config.ts"]);
}

#[test]
fn config_view_playwright_configs_none() {
    let cfg = NoMistakesConfig::default();
    let view = ConfigView::new(&cfg);
    assert!(view.playwright_configs().is_none());
}

#[test]
fn config_view_vitest_and_jest_configs() {
    let yaml = r#"
tests:
  vitest:
    configs: vitest.config.mts
  jest:
    configs:
      - jest.config.mjs
"#;
    let cfg: NoMistakesConfig = serde_yaml::from_str(yaml).unwrap();
    let view = ConfigView::new(&cfg);
    assert_eq!(view.vitest_configs().unwrap(), vec!["vitest.config.mts"]);
    assert_eq!(view.jest_configs().unwrap(), vec!["jest.config.mjs"]);
}

#[test]
fn config_view_test_id_attributes() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    let view = ConfigView::new(&cfg);
    assert_eq!(view.test_id_attributes(), &["data-testid", "data-pw"]);
}

#[test]
fn config_view_html_ids() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    let view = ConfigView::new(&cfg);
    assert!(!view.html_ids());
}

#[test]
fn config_view_component_selector_attributes() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    let view = ConfigView::new(&cfg);
    let attrs = view.component_selector_attributes();
    assert_eq!(attrs["dataPw"], "data-pw");
}

#[test]
fn config_view_selector_roots_and_exclude() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    let view = ConfigView::new(&cfg);
    assert_eq!(view.selector_roots(), &["web/app", "web/components"]);
    assert!(view
        .selector_exclude()
        .contains(&"**/*.test.tsx".to_string()));
}

#[test]
fn config_view_filesystem() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    let view = ConfigView::new(&cfg);
    assert_eq!(view.skip_directories(), &[".next", "node_modules"]);
}

#[test]
fn config_view_project_rules() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    let view = ConfigView::new(&cfg);
    assert!(view
        .project_rules("backend")
        .contains(&"http-route-static-paths"));
    assert!(view.project_rules("nonexistent").is_empty());
}

#[test]
fn config_view_rule_lookup() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    let view = ConfigView::new(&cfg);
    assert!(view.rule("http-route-static-paths").is_some());
    assert!(view.rule("nonexistent-rule").is_none());
}

#[test]
fn config_view_enabled_rules() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    let view = ConfigView::new(&cfg);
    let rules = view.enabled_rules_for("backend");
    assert!(rules.iter().any(|(id, _)| *id == "http-route-static-paths"));
}

#[test]
fn config_view_disabled_rule_excluded() {
    let cfg = load_v2_config(&fixture("disabled-rule"), None).unwrap();
    let view = ConfigView::new(&cfg);
    let rules = view.enabled_rules_for("backend");
    assert!(rules.iter().any(|(id, _)| *id == "active-rule"));
    assert!(!rules.iter().any(|(id, _)| *id == "disabled-rule"));
}

#[test]
fn duplicate_stems_errors() {
    let err = load_v2_config(&fixture("duplicate-stems"), None)
        .err()
        .unwrap();
    assert!(err.to_string().contains("multiple config files found"));
}

// ── detect_and_parse explicit tool paths ──────────────────────────────────────

#[test]
fn explicit_playwright_config_path_dispatched() {
    let dir = fixture("legacy-playwright");
    let explicit = dir.join(".playwright-ast-coverage.yaml");
    let cfg = load_v2_config(&dir, Some(&explicit)).unwrap();
    assert_eq!(cfg.projects["web"].type_, Some(ProjectType::Nextjs));
}

#[test]
fn explicit_react_traits_config_path_dispatched() {
    let dir = fixture("legacy-react-traits");
    let explicit = dir.join(".react-traits.yaml");
    let cfg = load_v2_config(&dir, Some(&explicit)).unwrap();
    assert_eq!(cfg.projects["web"].root.as_deref(), Some("src/app"));
}

#[test]
fn explicit_next_to_fetch_config_path_dispatched() {
    let dir = fixture("legacy-next-to-fetch");
    let explicit = dir.join(".next-to-fetch.yaml");
    let cfg = load_v2_config(&dir, Some(&explicit)).unwrap();
    assert_eq!(cfg.projects["web"].root.as_deref(), Some("app"));
}

// ── legacy defaults ───────────────────────────────────────────────────────────

#[test]
fn legacy_playwright_minimal_uses_defaults() {
    let cfg = load_v2_config(&fixture("legacy-playwright-minimal"), None).unwrap();
    assert_eq!(cfg.projects["web"].root.as_deref(), Some("app"));
    let pw = &cfg.tests.playwright;
    assert!(pw.selectors.test_ids.contains(&"data-testid".to_string()));
    assert!(pw.selectors.test_ids.contains(&"data-pw".to_string()));
    assert_eq!(pw.selector_roots, vec!["app"]);
}

#[test]
fn legacy_simple_no_frontend_root_returns_empty_projects() {
    let cfg = load_v2_config(&fixture("legacy-react-traits-minimal"), None).unwrap();
    assert!(cfg.projects.is_empty());
}

#[test]
fn legacy_guardrails_disabled_rule_converted() {
    let cfg = load_v2_config(&fixture("legacy-guardrails-disabled"), None).unwrap();
    let view = ConfigView::new(&cfg);
    let rules = view.enabled_rules_for("backend");
    assert!(rules.iter().any(|(id, _)| *id == "active-rule"));
    assert!(!rules.iter().any(|(id, _)| *id == "disabled-rule"));
}

#[test]
fn legacy_guardrails_project_rule_without_top_level_options_converted() {
    let cfg = load_v2_config(&fixture("legacy-guardrails-project-rule-only"), None).unwrap();
    assert!(cfg.rule_configured("unique-exports"));
    assert!(cfg
        .rule_applications("unique-exports")
        .iter()
        .any(|rule| rule.projects == vec!["app"]));
}

#[test]
fn config_view_rule_applications_are_project_scoped() {
    let cfg = load_v2_config(&fixture("project-unknown-rule"), None).unwrap();
    let view = ConfigView::new(&cfg);
    let rules = view.enabled_rules_for("backend");
    assert!(rules.iter().any(|(id, _)| *id == "known-rule"));
    assert!(rules.iter().any(|(id, _)| *id == "ghost-rule"));
}

// ── parse error propagation ───────────────────────────────────────────────────

#[test]
fn malformed_playwright_config_errors() {
    let dir = fixture("legacy-playwright-malformed");
    let explicit = dir.join(".playwright-ast-coverage.yaml");
    let err = load_v2_config(&dir, Some(&explicit)).err().unwrap();
    assert!(!err.to_string().is_empty());
}

#[test]
fn malformed_guardrails_config_errors() {
    let err = load_v2_config(&fixture("legacy-guardrails-malformed"), None)
        .err()
        .unwrap();
    assert!(!err.to_string().is_empty());
}

#[test]
fn malformed_react_traits_config_errors() {
    let err = load_v2_config(&fixture("legacy-react-traits-malformed"), None)
        .err()
        .unwrap();
    assert!(!err.to_string().is_empty());
}

// ── find_config_root ──────────────────────────────────────────────────────────

#[test]
fn find_config_root_v2_stem_returns_root() {
    let dir = fixture("basic");
    assert_eq!(find_config_root(&dir), dir);
}

#[test]
fn find_config_root_tool_stem_returns_root() {
    let dir = fixture("legacy-playwright");
    assert_eq!(find_config_root(&dir), dir);
}

#[test]
fn find_config_root_guardrails_returns_containing_dir() {
    let dir = fixture("legacy-guardrails");
    assert_eq!(find_config_root(&dir), dir);
}

#[test]
fn find_config_root_no_config_returns_start() {
    let dir = fixture("empty");
    assert_eq!(find_config_root(&dir), dir);
}
