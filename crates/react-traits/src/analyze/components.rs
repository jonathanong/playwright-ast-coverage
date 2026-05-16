mod helpers;

pub(crate) use helpers::is_class_component;
pub(crate) use helpers::is_component_expr;
use oxc_ast::ast::{
    BindingPattern, Declaration, ExportDefaultDeclarationKind, Expression, Program, Statement,
};
use oxc_span::Span;
use std::collections::HashMap;

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
    let mut local_vars: HashMap<&str, Span> = HashMap::new();
    for stmt in &program.body {
        match stmt {
            Statement::VariableDeclaration(v) => {
                for declarator in &v.declarations {
                    if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                        let name = id.name.as_ref();
                        if is_component_name(name) {
                            if let Some(init) = &declarator.init {
                                if is_component_expr(init) {
                                    local_vars.insert(name, declarator.span);
                                }
                            }
                        }
                    }
                }
            }
            Statement::FunctionDeclaration(f) if f.id.is_some() => {
                let id = f.id.as_ref().unwrap();
                let name = id.name.as_ref();
                if is_component_name(name) {
                    local_vars.insert(name, f.span);
                }
            }
            Statement::ClassDeclaration(c) if c.id.is_some() => {
                let id = c.id.as_ref().unwrap();
                if is_component_name(id.name.as_ref()) && is_class_component(c) {
                    local_vars.insert(id.name.as_ref(), c.span);
                }
            }
            _ => {}
        }
    }

    let mut components = Vec::new();

    for stmt in &program.body {
        match stmt {
            Statement::ExportDefaultDeclaration(export) => {
                let span = export.span;
                match &export.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                        components.push(ComponentDef {
                            name: "default".to_string(),
                            span: f.span,
                        });
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(c) if is_class_component(c) => {
                        components.push(ComponentDef {
                            name: "default".to_string(),
                            span: c.span,
                        });
                    }
                    ExportDefaultDeclarationKind::ArrowFunctionExpression(_)
                    | ExportDefaultDeclarationKind::FunctionExpression(_) => {
                        components.push(ComponentDef {
                            name: "default".to_string(),
                            span,
                        });
                    }
                    ExportDefaultDeclarationKind::CallExpression(call) => {
                        let callee_name = match &call.callee {
                            Expression::Identifier(id) => id.name.as_ref(),
                            Expression::StaticMemberExpression(m) if matches!(&m.object, Expression::Identifier(obj) if obj.name == "React") => {
                                m.property.name.as_ref()
                            }
                            _ => "",
                        };
                        if matches!(callee_name, "memo" | "forwardRef" | "lazy" | "dynamic") {
                            // If wrapping a named identifier, use that identifier's local span so
                            // span-scoped trait visitors capture hooks inside the wrapped function.
                            let component_span = if let Some(first_arg) = call.arguments.first() {
                                if let Some(Expression::Identifier(id)) = first_arg.as_expression()
                                {
                                    local_vars.get(id.name.as_ref()).copied().unwrap_or(span)
                                } else {
                                    span
                                }
                            } else {
                                span
                            };
                            components.push(ComponentDef {
                                name: "default".to_string(),
                                span: component_span,
                            });
                        }
                    }
                    ExportDefaultDeclarationKind::ParenthesizedExpression(p)
                        if is_component_expr(&p.expression) =>
                    {
                        components.push(ComponentDef {
                            name: "default".to_string(),
                            span,
                        });
                    }
                    ExportDefaultDeclarationKind::Identifier(id) => {
                        let name = id.name.as_ref();
                        if let Some(&var_span) = local_vars.get(name) {
                            components.push(ComponentDef {
                                name: "default".to_string(),
                                span: var_span,
                            });
                        }
                    }
                    _ => {}
                }
            }
            Statement::ExportNamedDeclaration(export) => {
                if let Some(decl) = &export.declaration {
                    match decl {
                        Declaration::FunctionDeclaration(f) if f.id.is_some() => {
                            let id = f.id.as_ref().unwrap();
                            let name = id.name.as_ref();
                            if is_component_name(name) {
                                components.push(ComponentDef {
                                    name: name.to_string(),
                                    span: f.span,
                                });
                            }
                        }
                        Declaration::VariableDeclaration(v) => {
                            for declarator in &v.declarations {
                                if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                                    let name = id.name.as_ref();
                                    if is_component_name(name) {
                                        if let Some(init) = &declarator.init {
                                            if is_component_expr(init) {
                                                components.push(ComponentDef {
                                                    name: name.to_string(),
                                                    span: declarator.span,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Declaration::ClassDeclaration(c) if c.id.is_some() => {
                            let id = c.id.as_ref().unwrap();
                            let name = id.name.as_ref();
                            if is_component_name(name) && is_class_component(c) {
                                components.push(ComponentDef {
                                    name: name.to_string(),
                                    span: c.span,
                                });
                            }
                        }
                        _ => {}
                    }
                } else {
                    // `export { Foo, Bar }` — resolve specifiers against local_vars
                    for spec in &export.specifiers {
                        let local_name = spec.local.name();
                        if let Some(&var_span) = local_vars.get(local_name.as_ref()) {
                            let exported_name = spec.exported.name();
                            components.push(ComponentDef {
                                name: exported_name.as_ref().to_string(),
                                span: var_span,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    components
}

#[cfg(test)]
mod tests;
