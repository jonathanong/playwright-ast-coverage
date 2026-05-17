use anyhow::Result;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use crate::config::parse_config;

use super::schema::{
    FilesystemConfig, NoMistakesConfig, PlaywrightSelectors, PlaywrightTestConfig, Project,
    ProjectType, RuleDef, StringOrList, Tests,
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
        suites: Vec::new(),
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
    let projects = gc
        .projects
        .into_iter()
        .map(|(name, gp)| {
            let root = gp.root.or(gp.root_path);
            (
                name,
                Project {
                    root,
                    rules: gp.rules,
                    ..Default::default()
                },
            )
        })
        .collect();

    let rules = gc
        .rules
        .into_iter()
        .map(|(id, opts)| {
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
            (
                id,
                RuleDef {
                    message,
                    enabled,
                    options: opts,
                },
            )
        })
        .collect();

    Ok(NoMistakesConfig {
        filesystem: gc.filesystem,
        projects,
        tests: Tests::default(),
        rules,
    })
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
