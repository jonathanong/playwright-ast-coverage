use crate::ast;
use anyhow::Result;
use oxc_ast::ast::{
    Argument, ArrayExpression, ArrayExpressionElement, AssignmentTarget, BindingPattern,
    ExportDefaultDeclarationKind, Expression, ObjectExpression, ObjectPropertyKind, Program,
    PropertyKey, Statement,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const DEFAULT_TEST_MATCH: &[&str] = &[
    "**/*.spec.ts",
    "**/*.spec.tsx",
    "**/*.spec.js",
    "**/*.spec.jsx",
    "**/*.spec.mts",
    "**/*.spec.cts",
    "**/*.spec.mjs",
    "**/*.spec.cjs",
    "**/*.test.ts",
    "**/*.test.tsx",
    "**/*.test.js",
    "**/*.test.jsx",
    "**/*.test.mts",
    "**/*.test.cts",
    "**/*.test.mjs",
    "**/*.test.cjs",
];
const DEFAULT_TEST_ID_ATTRIBUTE: &str = "data-testid";

pub struct PlaywrightConfig {
    pub name: Option<String>,
    pub projects: Vec<TestProject>,
}

pub struct TestProject {
    pub config_dir: PathBuf,
    pub test_dir: String,
    pub test_match: Vec<String>,
    pub test_ignore: Vec<String>,
    pub base_url: Option<String>,
    pub test_id_attribute: String,
}

#[derive(Default)]
struct ParsedOptions {
    name: Option<String>,
    test_dir: Option<String>,
    test_match: Option<Vec<String>>,
    test_ignore: Option<Vec<String>>,
    base_url: Option<String>,
    test_id_attribute: Option<String>,
}

impl PlaywrightConfig {
    pub fn base_urls(&self) -> Vec<String> {
        let mut urls: Vec<String> = self
            .projects
            .iter()
            .filter_map(|project| project.base_url.clone())
            .collect();
        urls.sort();
        urls.dedup();
        urls
    }

    pub fn test_id_attributes(&self) -> Vec<String> {
        let mut attributes: Vec<String> = self
            .projects
            .iter()
            .map(|project| project.test_id_attribute.clone())
            .collect();
        attributes.sort();
        attributes.dedup();
        attributes
    }
}

pub fn load_many(
    root: &Path,
    config_paths: &[PathBuf],
    config_name_filter: Option<&str>,
) -> Result<PlaywrightConfig> {
    if config_paths.is_empty() {
        if let Some(name) = config_name_filter {
            anyhow::bail!("--project requires a named Playwright config, but no config was found matching {name}");
        }
        return Ok(default_config(root));
    }

    let mut configs = Vec::new();
    for config_path in config_paths {
        let config = load(root, config_path)?;
        configs.push((config_path, config));
    }

    validate_config_names(&configs, config_name_filter)?;
    match config_name_filter {
        Some(name)
            if !configs
                .iter()
                .any(|(_, config)| config.name.as_deref() == Some(name)) =>
        {
            return Err(missing_config_name_error(name));
        }
        _ => {}
    }

    let mut projects = Vec::new();
    for (_, config) in configs {
        if config_name_filter.is_some_and(|name| config.name.as_deref() != Some(name)) {
            continue;
        }
        projects.extend(config.projects);
    }

    Ok(PlaywrightConfig {
        name: config_name_filter.map(str::to_string),
        projects,
    })
}

fn missing_config_name_error(name: &str) -> anyhow::Error {
    anyhow::Error::msg(format!("no Playwright config found with name {name}"))
}

impl TestProject {
    pub fn test_dir(&self, root: &Path) -> PathBuf {
        let path = Path::new(&self.test_dir);
        if path.is_absolute() {
            path.to_path_buf()
        } else if self.config_dir.is_absolute() {
            self.config_dir.join(path)
        } else {
            root.join(&self.config_dir).join(path)
        }
    }
}

pub fn load(root: &Path, config_path: &Path) -> Result<PlaywrightConfig> {
    if !config_path.exists() {
        anyhow::bail!(
            "Playwright config does not exist: {}",
            config_path.display()
        );
    }

    let source = std::fs::read_to_string(config_path)?;
    parse_from_path(&source, config_path, config_path.parent().unwrap_or(root))
}

fn default_config(root: &Path) -> PlaywrightConfig {
    PlaywrightConfig {
        name: None,
        projects: vec![TestProject {
            config_dir: root.to_path_buf(),
            test_dir: ".".to_string(),
            test_match: default_test_match(),
            test_ignore: Vec::new(),
            base_url: None,
            test_id_attribute: DEFAULT_TEST_ID_ATTRIBUTE.to_string(),
        }],
    }
}

#[cfg(test)]
fn parse(source: &str, config_dir: &Path) -> Result<PlaywrightConfig> {
    parse_from_path(source, Path::new("playwright.config.ts"), config_dir)
}

fn parse_from_path(source: &str, path: &Path, config_dir: &Path) -> Result<PlaywrightConfig> {
    ast::with_program(path, source, |program, source| {
        parse_program(program, source, config_dir)
    })?
}

fn parse_program(
    program: &Program<'_>,
    source: &str,
    config_dir: &Path,
) -> Result<PlaywrightConfig> {
    let bindings = top_level_object_bindings(program);
    let Some(root_object) = default_export_object(program) else {
        return Ok(PlaywrightConfig {
            name: None,
            projects: vec![merge_project(config_dir, &ParsedOptions::default(), None)],
        });
    };
    let root_options = parse_options(root_object, source, &bindings)?;
    let project_objects = project_objects(root_object);

    if project_objects.is_empty() {
        return Ok(PlaywrightConfig {
            name: root_options.name.clone(),
            projects: vec![merge_project(config_dir, &root_options, None)],
        });
    }

    let mut projects = Vec::new();
    for project_object in project_objects {
        projects.push(merge_project(
            config_dir,
            &root_options,
            Some(parse_options(project_object, source, &bindings)?),
        ));
    }

    Ok(PlaywrightConfig {
        name: root_options.name,
        projects,
    })
}

fn validate_config_names(
    configs: &[(&PathBuf, PlaywrightConfig)],
    config_name_filter: Option<&str>,
) -> Result<()> {
    if configs.len() <= 1 && config_name_filter.is_none() {
        return Ok(());
    }

    let mut seen = BTreeMap::new();
    for (path, config) in configs {
        let Some(name) = config.name.as_deref() else {
            anyhow::bail!(
                "Playwright config {} must define top-level name when multiple configs are analyzed or --project is used",
                path.display()
            );
        };
        if let Some(previous) = seen.insert(name.to_string(), path.display().to_string()) {
            anyhow::bail!(
                "Playwright config name {name} is duplicated by {} and {}",
                previous,
                path.display()
            );
        }
    }
    Ok(())
}

fn merge_project(
    config_dir: &Path,
    root: &ParsedOptions,
    project: Option<ParsedOptions>,
) -> TestProject {
    let project = project.unwrap_or_default();

    TestProject {
        config_dir: config_dir.to_path_buf(),
        test_dir: project
            .test_dir
            .or_else(|| root.test_dir.clone())
            .unwrap_or_else(|| ".".to_string()),
        test_match: project
            .test_match
            .or_else(|| root.test_match.clone())
            .unwrap_or_else(default_test_match),
        test_ignore: combine(root.test_ignore.clone(), project.test_ignore),
        base_url: project.base_url.or_else(|| root.base_url.clone()),
        test_id_attribute: project
            .test_id_attribute
            .or_else(|| root.test_id_attribute.clone())
            .unwrap_or_else(|| DEFAULT_TEST_ID_ATTRIBUTE.to_string()),
    }
}

fn parse_options(
    object: &ObjectExpression<'_>,
    source: &str,
    bindings: &BTreeMap<String, &Expression<'_>>,
) -> Result<ParsedOptions> {
    let use_object = property_expression(object, "use").and_then(|value| {
        let mut seen = BTreeSet::new();
        expression_config_object(value, bindings, &mut seen)
    });

    Ok(ParsedOptions {
        name: property_expression(object, "name").and_then(|value| optional_string(value, source)),
        test_dir: property_expression(object, "testDir")
            .map(|value| required_string(value, source, "testDir"))
            .transpose()?,
        test_match: property_expression(object, "testMatch")
            .map(|value| required_string_or_array(value, source, "testMatch"))
            .transpose()?,
        test_ignore: property_expression(object, "testIgnore")
            .map(|value| required_string_or_array(value, source, "testIgnore"))
            .transpose()?,
        base_url: use_object
            .and_then(|value| property_expression(value, "baseURL"))
            .or_else(|| property_expression(object, "baseURL"))
            .and_then(|value| optional_string(value, source)),
        test_id_attribute: use_object
            .and_then(|value| property_expression(value, "testIdAttribute"))
            .or_else(|| property_expression(object, "testIdAttribute"))
            .and_then(|value| optional_string(value, source)),
    })
}

fn combine(left: Option<Vec<String>>, right: Option<Vec<String>>) -> Vec<String> {
    let mut values = left.unwrap_or_default();
    values.extend(right.unwrap_or_default());
    values
}

fn default_test_match() -> Vec<String> {
    DEFAULT_TEST_MATCH
        .iter()
        .map(|pattern| pattern.to_string())
        .collect()
}

fn default_export_object<'a>(program: &'a Program<'a>) -> Option<&'a ObjectExpression<'a>> {
    let bindings = top_level_object_bindings(program);

    for statement in &program.body {
        if let Statement::ExportDefaultDeclaration(export) = statement {
            return export_config_object(&export.declaration, &bindings);
        }

        if let Some(object) = commonjs_config_object(statement, &bindings) {
            return Some(object);
        }
    }
    None
}

fn top_level_object_bindings<'a>(program: &'a Program<'a>) -> BTreeMap<String, &'a Expression<'a>> {
    let mut bindings = BTreeMap::new();
    for statement in &program.body {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        for declarator in &declaration.declarations {
            let (Some(name), Some(init)) =
                (binding_identifier_name(&declarator.id), &declarator.init)
            else {
                continue;
            };
            bindings.insert(name.to_string(), init);
        }
    }
    bindings
}

fn binding_identifier_name<'a>(binding: &'a BindingPattern<'a>) -> Option<&'a str> {
    match binding {
        BindingPattern::BindingIdentifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

fn export_config_object<'a>(
    export: &'a ExportDefaultDeclarationKind<'a>,
    bindings: &BTreeMap<String, &'a Expression<'a>>,
) -> Option<&'a ObjectExpression<'a>> {
    match export {
        ExportDefaultDeclarationKind::ObjectExpression(object) => Some(object),
        ExportDefaultDeclarationKind::CallExpression(call) => {
            call.arguments.first().and_then(|argument| {
                let mut seen = BTreeSet::new();
                argument_config_object(argument, bindings, &mut seen)
            })
        }
        ExportDefaultDeclarationKind::Identifier(identifier) => {
            let mut seen = BTreeSet::new();
            identifier_config_object(identifier.name.as_str(), bindings, &mut seen)
        }
        ExportDefaultDeclarationKind::ParenthesizedExpression(parenthesized) => {
            let mut seen = BTreeSet::new();
            expression_config_object(&parenthesized.expression, bindings, &mut seen)
        }
        _ => None,
    }
}

fn commonjs_config_object<'a>(
    statement: &'a Statement<'a>,
    bindings: &BTreeMap<String, &'a Expression<'a>>,
) -> Option<&'a ObjectExpression<'a>> {
    let Statement::ExpressionStatement(statement) = statement else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = &statement.expression else {
        return None;
    };
    if assignment_target_path(&assignment.left)
        .as_deref()
        .is_none_or(|parts| parts != ["module", "exports"])
    {
        return None;
    }
    let mut seen = BTreeSet::new();
    expression_config_object(&assignment.right, bindings, &mut seen)
}

fn assignment_target_path(target: &AssignmentTarget<'_>) -> Option<Vec<String>> {
    match target {
        AssignmentTarget::StaticMemberExpression(member) => {
            let mut parts = ast::expression_path(&member.object)?;
            parts.push(member.property.name.to_string());
            Some(parts)
        }
        _ => None,
    }
}

fn argument_config_object<'a>(
    argument: &'a Argument<'a>,
    bindings: &BTreeMap<String, &'a Expression<'a>>,
    seen: &mut BTreeSet<String>,
) -> Option<&'a ObjectExpression<'a>> {
    match argument {
        Argument::ObjectExpression(object) => Some(object),
        Argument::Identifier(identifier) => {
            identifier_config_object(identifier.name.as_str(), bindings, seen)
        }
        Argument::ParenthesizedExpression(parenthesized) => {
            expression_config_object(&parenthesized.expression, bindings, seen)
        }
        _ => None,
    }
}

fn expression_config_object<'a>(
    expression: &'a Expression<'a>,
    bindings: &BTreeMap<String, &'a Expression<'a>>,
    seen: &mut BTreeSet<String>,
) -> Option<&'a ObjectExpression<'a>> {
    match expression {
        Expression::ObjectExpression(object) => Some(object),
        Expression::Identifier(identifier) => {
            identifier_config_object(identifier.name.as_str(), bindings, seen)
        }
        Expression::CallExpression(call) => call
            .arguments
            .first()
            .and_then(|argument| argument_config_object(argument, bindings, seen)),
        Expression::ParenthesizedExpression(parenthesized) => {
            expression_config_object(&parenthesized.expression, bindings, seen)
        }
        _ => None,
    }
}

fn identifier_config_object<'a>(
    name: &str,
    bindings: &BTreeMap<String, &'a Expression<'a>>,
    seen: &mut BTreeSet<String>,
) -> Option<&'a ObjectExpression<'a>> {
    if !seen.insert(name.to_string()) {
        return None;
    }
    let object = bindings
        .get(name)
        .and_then(|expression| expression_config_object(expression, bindings, seen));
    seen.remove(name);
    object
}

fn property_expression<'a>(
    object: &'a ObjectExpression<'a>,
    name: &str,
) -> Option<&'a Expression<'a>> {
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            continue;
        };
        if property.computed || property.method {
            continue;
        }
        if property_key_name(&property.key).as_deref() == Some(name) {
            return Some(&property.value);
        }
    }
    None
}

fn property_key_name(key: &PropertyKey<'_>) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.to_string()),
        PropertyKey::StringLiteral(literal) => Some(literal.value.to_string()),
        _ => None,
    }
}

fn project_objects<'a>(root: &'a ObjectExpression<'a>) -> Vec<&'a ObjectExpression<'a>> {
    let Some(Expression::ArrayExpression(projects)) = property_expression(root, "projects") else {
        return Vec::new();
    };
    projects
        .elements
        .iter()
        .filter_map(array_element_object)
        .collect()
}

fn array_element_object<'a>(
    element: &'a ArrayExpressionElement<'a>,
) -> Option<&'a ObjectExpression<'a>> {
    match element {
        ArrayExpressionElement::ObjectExpression(object) => Some(object),
        _ => None,
    }
}

fn required_string(expression: &Expression<'_>, source: &str, name: &str) -> Result<String> {
    optional_string(expression, source)
        .ok_or_else(|| anyhow::anyhow!("expected string literal for {name}"))
}

fn optional_string(expression: &Expression<'_>, source: &str) -> Option<String> {
    match expression {
        Expression::StringLiteral(literal) => Some(literal.value.to_string()),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => {
            Some(ast::template_literal_text(template, source))
        }
        Expression::ParenthesizedExpression(parenthesized) => {
            optional_string(&parenthesized.expression, source)
        }
        _ => None,
    }
}

fn required_string_or_array(
    expression: &Expression<'_>,
    source: &str,
    name: &str,
) -> Result<Vec<String>> {
    if let Some(value) = optional_string(expression, source) {
        return Ok(vec![value]);
    }
    let Some(Expression::ArrayExpression(array)) = parenthesized_expression(expression) else {
        anyhow::bail!("expected string literal or string array for {name}");
    };
    string_array(array, source, name)
}

fn parenthesized_expression<'a>(expression: &'a Expression<'a>) -> Option<&'a Expression<'a>> {
    match expression {
        Expression::ParenthesizedExpression(parenthesized) => {
            parenthesized_expression(&parenthesized.expression)
        }
        _ => Some(expression),
    }
}

fn string_array(array: &ArrayExpression<'_>, source: &str, name: &str) -> Result<Vec<String>> {
    let mut values = Vec::new();
    let mut saw_regex = false;
    for element in &array.elements {
        match element {
            ArrayExpressionElement::StringLiteral(literal) => {
                values.push(literal.value.to_string())
            }
            ArrayExpressionElement::TemplateLiteral(template)
                if template.expressions.is_empty() =>
            {
                values.push(ast::template_literal_text(template, source));
            }
            ArrayExpressionElement::RegExpLiteral(_) => saw_regex = true,
            _ => {}
        }
    }
    if values.is_empty() && saw_regex {
        anyhow::bail!("regular-expression {name} patterns are not supported; use string globs");
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{fixture_path, fixture_source};

    #[test]
    fn parses_test_dir_and_match() {
        let source = fixture_source(&["playwright_config", "test-dir-and-match.ts"]);
        let parsed = parse(&source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.name, None);
        assert_eq!(parsed.projects[0].test_dir, "./tests/e2e");
        assert_eq!(parsed.projects[0].test_match, vec!["**/*.spec.ts"]);
    }

    #[test]
    fn parses_projects_with_inheritance() {
        let source = fixture_source(&["playwright_config", "projects-with-inheritance.ts"]);
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
        let source = fixture_source(&["playwright_config", "top-level-base-url.ts"]);
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
        let source = fixture_source(&["playwright_config", "default-identifier.ts"]);
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
        let source = fixture_source(&["playwright_config", "define-config-identifier.ts"]);
        let parsed = parse(&source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects[0].test_dir, "./define-config-tests");
        assert_eq!(parsed.projects[0].test_match, vec!["**/*.define-config.ts"]);
    }

    #[test]
    fn parses_commonjs_config_exports() {
        let source = fixture_source(&["playwright_config", "commonjs-object.cjs"]);
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

        let source = fixture_source(&["playwright_config", "commonjs-define-config.cjs"]);
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

    #[test]
    fn resolves_identifier_backed_use_object() {
        let source = fixture_source(&["playwright_config", "use-identifier.ts"]);
        let parsed = parse(&source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects[0].test_dir, "./use-identifier-tests");
        assert_eq!(
            parsed.projects[0].base_url.as_deref(),
            Some("http://localhost:6200")
        );
        assert_eq!(parsed.projects[0].test_id_attribute, "data-shared");
    }

    #[test]
    fn cyclic_identifier_configs_fall_back_without_recursing() {
        let source = fixture_source(&["playwright_config", "cyclic-config.ts"]);
        let parsed = parse(&source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects[0].test_dir, ".");
    }

    #[test]
    fn template_literals_use_cooked_text() {
        let source = fixture_source(&["playwright_config", "cooked-template.ts"]);
        let parsed = parse(&source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects[0].test_dir, r#"tests\e2e"#);
        assert_eq!(parsed.projects[0].test_match, vec![r#"**\/*.spec.ts"#]);
    }

    #[test]
    fn parser_handles_advanced_export_shapes() {
        let source = fixture_source(&["playwright_config", "advanced-export-shapes.ts"]);
        let parsed = parse(&source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects[0].test_dir, "./advanced-export-tests");
        assert_eq!(
            parsed.projects[0].test_match,
            vec!["**/*.advanced-export.ts"]
        );

        let source = fixture_source(&["playwright_config", "non-object-binding.ts"]);
        let parsed = parse(&source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects[0].test_dir, ".");
    }

    #[test]
    fn ignores_non_literal_optional_playwright_values() {
        let source = fixture_source(&["playwright_config", "nonliteral-optional-values.ts"]);
        let parsed = parse(&source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects[0].test_dir, "./tests");
        assert_eq!(parsed.projects[0].base_url, None);
        assert_eq!(parsed.projects[0].test_id_attribute, "data-testid");
    }

    #[test]
    fn load_without_config_uses_default_project() {
        let parsed = load_many(Path::new("/repo"), &[], None).unwrap();
        assert_eq!(parsed.projects[0].test_dir, ".");
        assert!(parsed.projects[0]
            .test_match
            .contains(&"**/*.spec.ts".to_string()));
    }

    #[test]
    fn load_many_without_configs_rejects_project_filter() {
        let err = load_many(Path::new("/repo"), &[], Some("storybook"))
            .err()
            .expect("expected project filter without config to fail");
        assert!(err.to_string().contains("--project requires"));
    }

    #[test]
    fn load_missing_config_errors() {
        let err = load(Path::new("/repo"), Path::new("/repo/missing.ts"))
            .err()
            .expect("expected missing config to fail");
        assert!(err.to_string().contains("does not exist"));
    }

    #[test]
    fn load_many_errors_when_project_filter_matches_no_config() {
        let dir = fixture_path(&["config", "multi-playwright-config"]);
        let config = dir.join("playwright.config.mts");
        let err = load_many(&dir, &[config], Some("missing"))
            .err()
            .expect("expected missing config name to fail");
        assert!(err.to_string().contains("no Playwright config found"));
    }

    #[test]
    fn load_existing_config_reads_and_parses() {
        let dir = fixture_path(&["playwright_config", "load-existing"]);
        let config = dir.join("playwright.config.ts");
        let parsed = load(&dir, &config).unwrap();
        assert_eq!(parsed.projects[0].test_dir, "./tests");
    }

    #[test]
    fn load_directory_config_path_returns_read_error() {
        let dir = fixture_path(&["playwright_config", "load-existing"]);
        let err = load(&dir, &dir)
            .err()
            .expect("expected directory config path to fail");
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn test_dir_resolves_absolute_relative_and_relative_config_dir() {
        let absolute = TestProject {
            config_dir: PathBuf::from("/repo"),
            test_dir: "/tmp/tests".to_string(),
            test_match: vec![],
            test_ignore: vec![],
            base_url: None,
            test_id_attribute: "data-testid".to_string(),
        };
        assert_eq!(
            absolute.test_dir(Path::new("/repo")),
            PathBuf::from("/tmp/tests")
        );

        let absolute_config_relative_test_dir = TestProject {
            config_dir: PathBuf::from("/repo"),
            test_dir: "tests".to_string(),
            test_match: vec![],
            test_ignore: vec![],
            base_url: None,
            test_id_attribute: "data-testid".to_string(),
        };
        assert_eq!(
            absolute_config_relative_test_dir.test_dir(Path::new("/repo")),
            PathBuf::from("/repo/tests")
        );

        let relative_config = TestProject {
            config_dir: PathBuf::from("config"),
            test_dir: "tests".to_string(),
            test_match: vec![],
            test_ignore: vec![],
            base_url: None,
            test_id_attribute: "data-testid".to_string(),
        };
        assert_eq!(
            relative_config.test_dir(Path::new("/repo")),
            PathBuf::from("/repo/config/tests")
        );
    }

    #[test]
    fn parse_accepts_spaced_property_and_escaped_string() {
        let source = fixture_source(&["playwright_config", "spaced-property.ts"]);
        let parsed = parse(&source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects[0].test_dir, r#"tests\e2e"#);
    }

    #[test]
    fn parser_rejects_unsupported_required_values() {
        assert!(parse("export default { testDir: 123 }", Path::new("/repo")).is_err());
        assert!(parse("export default { testIgnore: 123 }", Path::new("/repo")).is_err());
        assert!(parse(
            "export default { testMatch: [/.*\\.spec\\.ts/] }",
            Path::new("/repo")
        )
        .is_err());
        assert!(parse("export default { testMatch: 123 }", Path::new("/repo")).is_err());
        assert!(parse(
            "export default { projects: [{ testDir: 123 }] }",
            Path::new("/repo")
        )
        .is_err());
    }

    #[test]
    fn malformed_projects_value_falls_back_to_single_project() {
        let parsed = parse(
            "export default { projects: makeProjects() }",
            Path::new("/repo"),
        )
        .unwrap();
        assert_eq!(parsed.projects.len(), 1);
    }

    #[test]
    fn root_options_ignore_project_values() {
        let source = fixture_source(&["playwright_config", "project-values-only.ts"]);
        let parsed = parse(&source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects[0].test_dir, "./project-tests");
        assert_eq!(parsed.projects[0].test_match, vec!["**/*.project.ts"]);
    }

    #[test]
    fn parser_handles_ast_edge_shapes() {
        let source = fixture_source(&["playwright_config", "no-default-export.ts"]);
        let parsed = parse(&source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects[0].test_dir, ".");

        let source = fixture_source(&["playwright_config", "non-object-default.ts"]);
        let parsed = parse(&source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects[0].test_dir, ".");

        let source = fixture_source(&["playwright_config", "default-function.ts"]);
        let parsed = parse(&source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects[0].test_dir, ".");

        let source = fixture_source(&["playwright_config", "edge-shapes.ts"]);
        let parsed = parse(&source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects.len(), 1);
        assert_eq!(parsed.projects[0].test_dir, "./project-tests");
        assert_eq!(parsed.projects[0].test_match, vec!["**/*.project.ts"]);
        assert_eq!(parsed.projects[0].test_ignore, vec!["**/skip/**"]);
        assert_eq!(
            parsed.projects[0].base_url.as_deref(),
            Some("http://localhost:3000")
        );
        assert_eq!(parsed.projects[0].test_id_attribute, "data-test");
    }
}
