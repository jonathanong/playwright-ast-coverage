mod local_vars;

use super::{is_class_component, is_component_expr, is_component_name, ComponentDef};
use local_vars::collect_component_declarations;
pub(super) use local_vars::collect_local_vars;
use oxc_ast::ast::{
    Declaration, ExportDefaultDeclaration, ExportDefaultDeclarationKind, ExportNamedDeclaration,
    Expression,
};
use oxc_span::Span;
use std::collections::HashMap;

pub(super) fn extract_default_export(
    export: &ExportDefaultDeclaration<'_>,
    local_vars: &HashMap<&str, Span>,
) -> Option<ComponentDef> {
    let span = export.span;
    match &export.declaration {
        ExportDefaultDeclarationKind::FunctionDeclaration(f) => Some(ComponentDef {
            name: "default".to_string(),
            span: f.span,
        }),
        ExportDefaultDeclarationKind::ClassDeclaration(c) if is_class_component(c) => {
            Some(ComponentDef {
                name: "default".to_string(),
                span: c.span,
            })
        }
        ExportDefaultDeclarationKind::ArrowFunctionExpression(_)
        | ExportDefaultDeclarationKind::FunctionExpression(_) => Some(ComponentDef {
            name: "default".to_string(),
            span,
        }),
        ExportDefaultDeclarationKind::CallExpression(call) => {
            extract_default_call_export(call, local_vars, span)
        }
        ExportDefaultDeclarationKind::ParenthesizedExpression(p)
            if is_component_expr(&p.expression) =>
        {
            Some(ComponentDef {
                name: "default".to_string(),
                span,
            })
        }
        ExportDefaultDeclarationKind::Identifier(id) => {
            let name = id.name.as_ref();
            local_vars.get(name).map(|&var_span| ComponentDef {
                name: "default".to_string(),
                span: var_span,
            })
        }
        _ => None,
    }
}

fn extract_default_call_export(
    call: &oxc_ast::ast::CallExpression<'_>,
    local_vars: &HashMap<&str, Span>,
    span: Span,
) -> Option<ComponentDef> {
    let callee_name = match &call.callee {
        Expression::Identifier(id) => id.name.as_ref(),
        Expression::StaticMemberExpression(m) if matches!(&m.object, Expression::Identifier(obj) if obj.name == "React") => {
            m.property.name.as_ref()
        }
        _ => "",
    };
    if !matches!(callee_name, "memo" | "forwardRef" | "lazy" | "dynamic") {
        return None;
    }

    let component_span = call
        .arguments
        .first()
        .and_then(|arg| arg.as_expression())
        .and_then(|expr| match expr {
            Expression::Identifier(id) => local_vars.get(id.name.as_ref()).copied(),
            _ => None,
        })
        .unwrap_or(span);

    Some(ComponentDef {
        span: component_span,
        name: "default".to_string(),
    })
}

pub(super) fn extract_named_export(
    export: &ExportNamedDeclaration<'_>,
    local_vars: &HashMap<&str, Span>,
    components: &mut Vec<ComponentDef>,
) {
    if let Some(decl) = &export.declaration {
        extract_named_declaration(decl, components);
    } else {
        extract_named_specifiers(export, local_vars, components);
    }
}

fn extract_named_declaration(decl: &Declaration<'_>, components: &mut Vec<ComponentDef>) {
    match decl {
        Declaration::FunctionDeclaration(f) => {
            if let Some(id) = &f.id {
                let name = id.name.as_ref();
                if is_component_name(name) {
                    components.push(ComponentDef {
                        name: name.to_string(),
                        span: f.span,
                    });
                }
            }
        }
        Declaration::VariableDeclaration(v) => {
            collect_component_declarations(v, |name, span| {
                components.push(ComponentDef {
                    name: name.to_string(),
                    span,
                });
            });
        }
        Declaration::ClassDeclaration(c) => {
            if let Some(id) = &c.id {
                let name = id.name.as_ref();
                if is_component_name(name) && is_class_component(c) {
                    components.push(ComponentDef {
                        name: name.to_string(),
                        span: c.span,
                    });
                }
            }
        }
        _ => {}
    }
}

fn extract_named_specifiers(
    export: &ExportNamedDeclaration<'_>,
    local_vars: &HashMap<&str, Span>,
    components: &mut Vec<ComponentDef>,
) {
    // `export { Foo, Bar }` resolves specifiers against local component declarations.
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
