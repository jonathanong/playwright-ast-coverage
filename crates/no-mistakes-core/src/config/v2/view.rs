use std::collections::BTreeMap;

use super::schema::{NoMistakesConfig, Project, ProjectType, RuleDef};

/// A read-only lens over a [`NoMistakesConfig`] that surfaces the effective
/// settings for a specific tool or domain without mutating the underlying
/// config.
pub struct ConfigView<'a> {
    config: &'a NoMistakesConfig,
}

impl<'a> ConfigView<'a> {
    pub fn new(config: &'a NoMistakesConfig) -> Self {
        Self { config }
    }

    /// Projects of the given `type_`, or all projects when `type_` is `None`.
    pub fn projects_of_type(&self, type_: Option<&ProjectType>) -> Vec<(&str, &Project)> {
        self.config
            .projects
            .iter()
            .filter(|(_, p)| match type_ {
                None => true,
                Some(t) => p.type_.as_ref() == Some(t),
            })
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    /// Server route definition globs, normalized relative to the config root.
    pub fn server_route_globs(&self) -> Vec<String> {
        let mut routes = Vec::new();
        for project in self.config.projects.values() {
            if project.routes.is_empty() {
                continue;
            }
            if project
                .type_
                .as_ref()
                .is_some_and(|type_| type_ != &ProjectType::Server)
            {
                continue;
            }
            let root = project.root.as_deref().unwrap_or(".");
            for route in &project.routes {
                routes.push(project_relative_glob(root, route));
            }
        }
        routes.sort();
        routes.dedup();
        routes
    }

    /// The first `nextjs` project's root path (or `"app"` if none configured).
    pub fn nextjs_root(&self) -> &str {
        self.config
            .projects
            .values()
            .find(|p| p.type_ == Some(ProjectType::Nextjs))
            .and_then(|p| p.root.as_deref())
            .unwrap_or("app")
    }

    /// Playwright config file glob(s), or `None` when not configured.
    pub fn playwright_configs(&self) -> Option<Vec<String>> {
        self.config
            .tests
            .playwright
            .configs
            .as_ref()
            .map(|c| c.values())
    }

    /// Vitest config file glob(s), or `None` when not configured.
    pub fn vitest_configs(&self) -> Option<Vec<String>> {
        self.config
            .tests
            .vitest
            .configs
            .as_ref()
            .map(|c| c.values())
    }

    /// Jest config file glob(s), or `None` when not configured.
    pub fn jest_configs(&self) -> Option<Vec<String>> {
        self.config.tests.jest.configs.as_ref().map(|c| c.values())
    }

    /// Test-ID selector attributes (e.g. `["data-testid", "data-pw"]`).
    pub fn test_id_attributes(&self) -> &[String] {
        &self.config.tests.playwright.selectors.test_ids
    }

    /// Whether HTML `id` attributes are tracked as selectors.
    pub fn html_ids(&self) -> bool {
        self.config.tests.playwright.selectors.html_ids
    }

    /// Component-prop → HTML-attribute mapping for selector tracking.
    pub fn component_selector_attributes(&self) -> &BTreeMap<String, String> {
        &self.config.tests.playwright.selectors.component_test_ids
    }

    /// Roots that are scanned for selector usage.
    pub fn selector_roots(&self) -> &[String] {
        &self.config.tests.playwright.selector_roots
    }

    /// Glob patterns excluded from selector scanning.
    pub fn selector_exclude(&self) -> &[String] {
        &self.config.tests.playwright.selector_exclude
    }

    /// Skip directories from the filesystem config.
    pub fn skip_directories(&self) -> &[String] {
        &self.config.filesystem.skip_directories
    }

    /// Skip file patterns from the filesystem config.
    pub fn skip_file_patterns(&self) -> &[String] {
        &self.config.filesystem.skip_file_patterns
    }

    /// Rules enabled for a named project (returns empty slice if unknown).
    pub fn project_rules(&self, project: &str) -> &[String] {
        self.config
            .projects
            .get(project)
            .map(|p| p.rules.as_slice())
            .unwrap_or(&[])
    }

    /// Look up a rule definition by ID.
    pub fn rule(&self, id: &str) -> Option<&RuleDef> {
        self.config.rules.get(id)
    }

    /// All rule IDs enabled for a project that have a top-level definition.
    pub fn enabled_rules_for(&self, project: &str) -> Vec<(&str, &RuleDef)> {
        self.project_rules(project)
            .iter()
            .filter_map(|id| {
                let def = self.config.rules.get(id.as_str())?;
                if def.enabled {
                    Some((id.as_str(), def))
                } else {
                    None
                }
            })
            .collect()
    }
}

fn project_relative_glob(root: &str, pattern: &str) -> String {
    let root = root.trim().trim_matches('/').trim_start_matches("./");
    let pattern = pattern
        .trim()
        .trim_start_matches('/')
        .trim_start_matches("./");
    if root.is_empty() || root == "." || pattern.starts_with(&format!("{root}/")) {
        pattern.to_string()
    } else {
        format!("{root}/{pattern}")
    }
}
