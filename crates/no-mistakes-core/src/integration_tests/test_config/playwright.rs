use super::shared;
use crate::ast;
use crate::integration_tests::project_config::prefix_globs;
use crate::integration_tests::types::ConfigProject;
use anyhow::Result;
use oxc_ast::ast::{ObjectExpression, Program};
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

pub(in crate::integration_tests) struct ParsedPlaywrightConfig {
    projects: Vec<PlaywrightProject>,
}

pub(in crate::integration_tests) struct PlaywrightProject {
    name: Option<String>,
    config_dir: PathBuf,
    test_dir: String,
    test_match: Vec<String>,
    test_ignore: Vec<String>,
}

#[derive(Default, Clone)]
struct Options {
    name: Option<String>,
    test_dir: Option<String>,
    test_match: Option<Vec<String>>,
    test_ignore: Option<Vec<String>>,
}

pub(in crate::integration_tests) fn parse_from_path(
    source: &str,
    path: &Path,
    config_dir: &Path,
) -> Result<ParsedPlaywrightConfig> {
    ast::with_program(path, source, |program, source| {
        parse_program(program, source, config_dir)
    })?
}

impl ParsedPlaywrightConfig {
    pub(in crate::integration_tests) fn into_projects(
        self,
        root: &Path,
        config: &str,
    ) -> Vec<ConfigProject> {
        self.projects
            .into_iter()
            .map(|project| {
                let test_dir = project.test_dir(root);
                ConfigProject {
                    config: Some(config.to_string()),
                    name: project.name,
                    include: prefix_globs(root, &test_dir, &project.test_match),
                    exclude: prefix_globs(root, &test_dir, &project.test_ignore),
                }
            })
            .collect()
    }
}

fn parse_program(
    program: &Program<'_>,
    source: &str,
    config_dir: &Path,
) -> Result<ParsedPlaywrightConfig> {
    let bindings = shared::top_level_object_bindings(program);
    let Some(root_object) = shared::default_export_object(program, &bindings, true) else {
        return Ok(single_project(config_dir, &Options::default(), None));
    };
    let root_options = parse_options(root_object, source)?;
    let project_objects = shared::project_objects(root_object);
    if project_objects.is_empty() {
        return Ok(single_project(config_dir, &root_options, None));
    }
    let mut projects = Vec::new();
    for project_object in project_objects {
        projects.push(merge_project(
            config_dir,
            &root_options,
            Some(parse_options(project_object, source)?),
        ));
    }
    Ok(ParsedPlaywrightConfig { projects })
}

fn single_project(
    config_dir: &Path,
    root: &Options,
    project: Option<Options>,
) -> ParsedPlaywrightConfig {
    ParsedPlaywrightConfig {
        projects: vec![merge_project(config_dir, root, project)],
    }
}

impl PlaywrightProject {
    fn test_dir(&self, root: &Path) -> PathBuf {
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

fn merge_project(config_dir: &Path, root: &Options, project: Option<Options>) -> PlaywrightProject {
    let project = project.unwrap_or_default();
    PlaywrightProject {
        name: project.name.or_else(|| root.name.clone()),
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
    }
}

fn parse_options(object: &ObjectExpression<'_>, source: &str) -> Result<Options> {
    Ok(Options {
        name: shared::property_expression(object, "name")
            .and_then(|value| shared::optional_string(value, source)),
        test_dir: string_property(object, source, "testDir")?,
        test_match: string_array_property(object, source, "testMatch")?,
        test_ignore: string_array_property(object, source, "testIgnore")?,
    })
}

fn string_property(
    object: &ObjectExpression<'_>,
    source: &str,
    name: &str,
) -> Result<Option<String>> {
    shared::property_expression(object, name)
        .map(|value| shared::required_string(value, source, name))
        .transpose()
}

fn string_array_property(
    object: &ObjectExpression<'_>,
    source: &str,
    name: &str,
) -> Result<Option<Vec<String>>> {
    shared::property_expression(object, name)
        .map(|value| {
            let values = shared::inferred_string_or_array(value, source, name)?;
            if values.is_empty() && name != "testIgnore" {
                anyhow::bail!("expected string literal or string array for {name}");
            }
            Ok(values)
        })
        .transpose()
}

fn default_test_match() -> Vec<String> {
    DEFAULT_TEST_MATCH
        .iter()
        .map(|glob| glob.to_string())
        .collect()
}

fn combine(left: Option<Vec<String>>, right: Option<Vec<String>>) -> Vec<String> {
    let mut values = left.unwrap_or_default();
    values.extend(right.unwrap_or_default());
    values
}
