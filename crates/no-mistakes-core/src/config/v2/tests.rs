use std::path::Path;

use super::discover::load_v2_config;
use super::schema::{NoMistakesConfig, ProjectType, StringOrList};
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

#[test]
fn empty_config_returns_default() {
    let cfg = load_v2_config(&fixture("empty"), None).unwrap();
    assert_eq!(cfg, NoMistakesConfig::default());
}

#[test]
fn missing_dir_returns_default() {
    let cfg = load_v2_config(Path::new("/tmp/no-mistakes-test-nonexistent-xyz"), None).unwrap();
    assert_eq!(cfg, NoMistakesConfig::default());
}

#[test]
fn basic_v2_config_parsed() {
    let cfg = load_v2_config(&fixture("basic"), None).unwrap();
    assert!(cfg.projects.contains_key("backend"));
    let backend = &cfg.projects["backend"];
    assert_eq!(backend.type_, Some(ProjectType::Server));
    assert_eq!(backend.root.as_deref(), Some("backend"));
    assert_eq!(backend.rules, vec!["http-route-static-paths"]);
    assert!(cfg.rules.contains_key("http-route-static-paths"));
}

#[test]
fn multi_project_config_parsed() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    assert!(cfg.projects.contains_key("backend"));
    assert!(cfg.projects.contains_key("web"));
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

#[test]
fn legacy_playwright_converted() {
    let cfg = load_v2_config(&fixture("legacy-playwright"), None).unwrap();
    assert!(cfg.projects.contains_key("web"));
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
    assert!(cfg.projects.contains_key("backend"));
    assert!(cfg.projects.contains_key("web"));
    assert_eq!(cfg.projects["backend"].root.as_deref(), Some("backend"));
    assert_eq!(
        cfg.projects["backend"].rules,
        vec!["http-route-static-paths"]
    );
    assert_eq!(
        cfg.filesystem.skip_directories,
        vec![".next", "node_modules"]
    );
    assert!(cfg.rules.contains_key("http-route-static-paths"));
}

#[test]
fn explicit_config_path_overrides_discovery() {
    let dir = fixture("multi-project");
    let explicit = dir.join(".no-mistakes.yml");
    let cfg = load_v2_config(&dir, Some(&explicit)).unwrap();
    assert!(cfg.projects.contains_key("web"));
}

#[test]
fn explicit_nonexistent_config_errors() {
    let dir = fixture("basic");
    let err = load_v2_config(&dir, Some(Path::new("nonexistent.yml")))
        .err()
        .unwrap();
    assert!(err.to_string().contains("does not exist"));
}

#[test]
fn config_view_nextjs_root() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    let view = ConfigView::new(&cfg);
    assert_eq!(view.nextjs_root(), "web");
}

#[test]
fn config_view_project_rules() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    let view = ConfigView::new(&cfg);
    assert!(view
        .project_rules("backend")
        .contains(&"http-route-static-paths".to_string()));
    assert!(view.project_rules("nonexistent").is_empty());
}

#[test]
fn config_view_enabled_rules() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    let view = ConfigView::new(&cfg);
    let rules = view.enabled_rules_for("backend");
    assert!(!rules.is_empty());
    assert!(rules.iter().any(|(id, _)| *id == "http-route-static-paths"));
}

#[test]
fn config_view_playwright_selectors() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    let view = ConfigView::new(&cfg);
    assert_eq!(view.test_id_attributes(), &["data-testid", "data-pw"]);
    assert!(!view.html_ids());
    assert_eq!(view.selector_roots(), &["web/app", "web/components"]);
}

#[test]
fn rule_def_options_deserialized() {
    let cfg = load_v2_config(&fixture("multi-project"), None).unwrap();
    let rule = cfg.rules.get("http-route-static-paths").unwrap();
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
