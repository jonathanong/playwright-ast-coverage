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

fn is_component_name(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_uppercase())
}

pub(crate) fn extract_components(program: &Program<'_>) -> Vec<ComponentDef> {
    // Collect top-level `const X = <component expr>` declarations for resolving
    // `export default X` re-exports (common pattern: const Page = () => ...; export default Page).
    let mut local_vars: HashMap<&str, Span> = HashMap::new();
    for stmt in &program.body {
        if let Statement::VariableDeclaration(v) = stmt {
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
                    ExportDefaultDeclarationKind::ClassDeclaration(c) => {
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
                            Expression::StaticMemberExpression(m) => m.property.name.as_ref(),
                            _ => "",
                        };
                        if matches!(callee_name, "memo" | "forwardRef" | "lazy") {
                            components.push(ComponentDef {
                                name: "default".to_string(),
                                span,
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
                        _ => {}
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

pub(crate) fn is_component_expr(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => true,
        Expression::CallExpression(call) => {
            let name = match &call.callee {
                Expression::Identifier(id) => id.name.as_ref(),
                Expression::StaticMemberExpression(m) => m.property.name.as_ref(),
                _ => return false,
            };
            matches!(name, "memo" | "forwardRef" | "lazy")
        }
        Expression::ParenthesizedExpression(p) => is_component_expr(&p.expression),
        _ => false,
    }
}
