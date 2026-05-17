use super::shared;
use crate::ast;
use crate::integration_tests::project_config::prefix_globs;
use crate::integration_tests::types::ConfigProject;
use anyhow::Result;
use oxc_ast::ast::{ObjectExpression, Program};
use std::path::Path;

const DEFAULT_INCLUDE: &[&str] = &[
    "**/*.{test,spec}.?(c|m)[jt]s?(x)",
    "**/__tests__/**/*.?(c|m)[jt]s?(x)",
];

#[derive(Default, Clone)]
struct Options {
    name: Option<String>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
}

pub(in crate::integration_tests) fn parse_from_path(
    source: &str,
    path: &Path,
    config_dir: &Path,
    root: &Path,
) -> Result<Vec<ConfigProject>> {
    ast::with_program(path, source, |program, source| {
        parse_program(program, source, config_dir, root)
    })?
}

fn parse_program(
    program: &Program<'_>,
    source: &str,
    config_dir: &Path,
    root: &Path,
) -> Result<Vec<ConfigProject>> {
    let bindings = shared::top_level_object_bindings(program);
    let Some(root_object) = shared::default_export_object(program, &bindings, false) else {
        return Ok(Vec::new());
    };
    let test_object =
        shared::property_object(root_object, "test", &bindings).unwrap_or(root_object);
    let root_options = parse_options(test_object, source)?;
    let project_objects = shared::project_objects(test_object);
    let mut projects = Vec::new();
    if project_objects.is_empty() {
        projects.push(to_project(config_dir, root, root_options));
        return Ok(projects);
    }

    for project_object in project_objects {
        let nested_test =
            shared::property_object(project_object, "test", &bindings).unwrap_or(project_object);
        let project_options = parse_options(nested_test, source)?;
        projects.push(to_project(
            config_dir,
            root,
            merge_options(&root_options, project_options),
        ));
    }
    Ok(projects)
}

fn to_project(config_dir: &Path, root: &Path, options: Options) -> ConfigProject {
    let include = options.include.unwrap_or_else(|| {
        DEFAULT_INCLUDE
            .iter()
            .map(|glob| glob.to_string())
            .collect()
    });
    ConfigProject {
        config: None,
        name: options.name,
        include: prefix_globs(root, config_dir, &include),
        exclude: prefix_globs(root, config_dir, &options.exclude.unwrap_or_default()),
    }
}

fn merge_options(root: &Options, project: Options) -> Options {
    Options {
        name: project.name.or_else(|| root.name.clone()),
        include: project.include.or_else(|| root.include.clone()),
        exclude: combine(root.exclude.clone(), project.exclude),
    }
}

fn combine(left: Option<Vec<String>>, right: Option<Vec<String>>) -> Option<Vec<String>> {
    let mut values = left.unwrap_or_default();
    values.extend(right.unwrap_or_default());
    (!values.is_empty()).then_some(values)
}

fn parse_options(object: &ObjectExpression<'_>, source: &str) -> Result<Options> {
    Ok(Options {
        name: shared::property_expression(object, "name")
            .and_then(|value| shared::optional_string(value, source)),
        include: string_array_property(object, source, "include")?,
        exclude: string_array_property(object, source, "exclude")?,
    })
}

fn string_array_property(
    object: &ObjectExpression<'_>,
    source: &str,
    name: &str,
) -> Result<Option<Vec<String>>> {
    shared::property_expression(object, name)
        .map(|value| shared::required_string_or_array(value, source, name))
        .transpose()
}
