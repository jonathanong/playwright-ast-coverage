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

    /// Rules enabled for a named project.
    pub fn project_rules(&self, project: &str) -> Vec<&str> {
        self.config
            .rules
            .iter()
            .filter(|rule| rule.applies_to_project(project))
            .map(|rule| rule.rule.as_str())
            .collect()
    }

    /// Look up the first enabled rule application by rule ID.
    pub fn rule(&self, id: &str) -> Option<&RuleDef> {
        self.config.rule_applications(id).into_iter().next()
    }

    /// All rule applications enabled for a project.
    pub fn enabled_rules_for(&self, project: &str) -> Vec<(&str, &RuleDef)> {
        self.config
            .rules
            .iter()
            .filter_map(|rule| {
                rule.applies_to_project(project)
                    .then_some((rule.rule.as_str(), rule))
            })
            .collect()
    }
}
