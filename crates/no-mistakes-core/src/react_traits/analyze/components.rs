mod extract;
mod helpers;

use extract::{collect_local_vars, extract_default_export, extract_named_export};
pub(crate) use helpers::is_class_component;
pub(crate) use helpers::is_component_expr;
use oxc_ast::ast::{Program, Statement};
use oxc_span::Span;

#[derive(Debug, Clone)]
pub(crate) struct ComponentDef {
    pub(crate) name: String,
    #[allow(dead_code)]
    pub(crate) span: Span,
}

pub(crate) fn is_component_name(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_uppercase())
}

pub(crate) fn extract_components(program: &Program<'_>) -> Vec<ComponentDef> {
    // First pass: collect top-level component variable and class declarations for resolving
    // `export default X` and `export { X }` re-exports.
    let local_vars = collect_local_vars(program);

    let mut components = Vec::new();

    for stmt in &program.body {
        match stmt {
            Statement::ExportDefaultDeclaration(export) => {
                if let Some(def) = extract_default_export(export, &local_vars) {
                    components.push(def);
                }
            }
            Statement::ExportNamedDeclaration(export) => {
                extract_named_export(export, &local_vars, &mut components);
            }
            _ => {}
        }
    }

    components
}

#[cfg(test)]
mod tests;
