use anyhow::Result;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use crate::config::parse_config;

use super::schema::{
    FilesystemConfig, NoMistakesConfig, PlaywrightSelectors, PlaywrightTestConfig, Project,
    ProjectType, RuleDef, RuleScope, StringOrList, Tests,
};
use super::ToolKind;

// ── playwright-ast-coverage legacy ──────────────────────────────────────────

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct PlaywrightRootConfig {
    #[serde(flatten)]
    legacy: PlaywrightFileConfig,
    playwright_ast_coverage: Option<PlaywrightFileConfig>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct PlaywrightFileConfig {
    frontend_root: Option<String>,
    playwright_config: Option<StringOrList>,
    selector_attributes: Option<Vec<String>>,
    component_selector_attributes: BTreeMap<String, String>,
    html_ids: bool,
    selector_roots: Option<Vec<String>>,
    selector_exclude: Vec<String>,
}

fn playwright_to_v2(source: &str, path: &Path) -> Result<NoMistakesConfig> {
    let root_cfg: PlaywrightRootConfig = parse_config(source, path)?;
    let fc = root_cfg.playwright_ast_coverage.unwrap_or(root_cfg.legacy);
    let frontend_root = fc.frontend_root.unwrap_or_else(|| "app".to_string());
    let selector_roots = fc
        .selector_roots
        .unwrap_or_else(|| vec![frontend_root.clone()]);
    let test_ids = fc
        .selector_attributes
        .unwrap_or_else(|| vec!["data-testid".to_string(), "data-pw".to_string()]);

    let mut cfg = NoMistakesConfig::default();
    cfg.projects.insert(
        "web".to_string(),
        Project {
            type_: Some(ProjectType::Nextjs),
            root: Some(frontend_root),
            ..Default::default()
        },
    );
    cfg.tests.playwright = PlaywrightTestConfig {
        configs: fc.playwright_config,
        projects: BTreeMap::new(),
        selectors: PlaywrightSelectors {
            html_ids: fc.html_ids,
            test_ids,
            component_test_ids: fc.component_selector_attributes,
        },
        selector_roots,
        selector_exclude: fc.selector_exclude,
    };
    Ok(cfg)
}

// ── .guardrailsrc legacy ─────────────────────────────────────────────────────

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct GuardrailsConfig {
    filesystem: FilesystemConfig,
    projects: BTreeMap<String, GuardrailsProject>,
    rules: HashMap<String, serde_yaml::Value>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct GuardrailsProject {
    root_path: Option<String>,
    root: Option<String>,
    rules: Vec<String>,
}

fn guardrails_to_v2(source: &str, path: &Path) -> Result<NoMistakesConfig> {
    let gc: GuardrailsConfig = parse_config(source, path)?;
    let mut project_rule_refs = Vec::new();
    let projects = gc
        .projects
        .into_iter()
        .map(|(name, gp)| {
            let root = gp.root.or(gp.root_path);
            for rule in gp.rules {
                project_rule_refs.push((name.clone(), rule));
            }
            (
                name,
                Project {
                    root,
                    ..Default::default()
                },
            )
        })
        .collect();

    let rule_options = gc
        .rules
        .into_iter()
        .map(|(id, opts)| (id, legacy_rule_def_parts(opts)))
        .collect::<HashMap<_, _>>();

    let mut referenced_rules = std::collections::HashSet::new();
    let mut rules = Vec::new();
    for (project, rule_id) in project_rule_refs {
        referenced_rules.insert(rule_id.clone());
        let (enabled, message, options) = rule_options
            .get(&rule_id)
            .cloned()
            .unwrap_or_else(default_legacy_rule_parts);
        rules.push(RuleDef {
            rule: rule_id,
            message,
            enabled,
            projects: vec![project],
            options,
            ..Default::default()
        });
    }
    for (rule_id, (enabled, message, options)) in rule_options {
        if referenced_rules.contains(&rule_id) {
            continue;
        }
        rules.push(RuleDef {
            rule: rule_id,
            message,
            enabled,
            scope: Some(RuleScope::Repository),
            options,
            ..Default::default()
        });
    }

    Ok(NoMistakesConfig {
        filesystem: gc.filesystem,
        projects,
        tests: Tests::default(),
        rules,
    })
}

fn legacy_rule_def_parts(opts: serde_yaml::Value) -> (bool, Option<String>, serde_yaml::Value) {
    let enabled = opts
        .as_mapping()
        .and_then(|m| m.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let message = opts
        .as_mapping()
        .and_then(|m| m.get("message"))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    (enabled, message, opts)
}

fn default_legacy_rule_parts() -> (bool, Option<String>, serde_yaml::Value) {
    (
        true,
        None,
        serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
    )
}

// ── react-traits / next-to-fetch legacy ──────────────────────────────────────

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SimpleLegacyConfig {
    frontend_root: Option<String>,
}

fn simple_to_v2(source: &str, path: &Path, project_type: ProjectType) -> Result<NoMistakesConfig> {
    let lc: SimpleLegacyConfig = parse_config(source, path)?;
    let mut cfg = NoMistakesConfig::default();
    if let Some(root) = lc.frontend_root {
        cfg.projects.insert(
            "web".to_string(),
            Project {
                type_: Some(project_type),
                root: Some(root),
                ..Default::default()
            },
        );
    }
    Ok(cfg)
}

// ── public dispatch ───────────────────────────────────────────────────────────

pub fn from_tool_config(source: &str, path: &Path, kind: ToolKind) -> Result<NoMistakesConfig> {
    match kind {
        ToolKind::Playwright => playwright_to_v2(source, path),
        ToolKind::ReactTraits => simple_to_v2(source, path, ProjectType::Nextjs),
        ToolKind::NextToFetch => simple_to_v2(source, path, ProjectType::Nextjs),
    }
}

pub fn from_guardrails_config(source: &str, path: &Path) -> Result<NoMistakesConfig> {
    guardrails_to_v2(source, path)
}
