use oxc_ast::ast::{Class, Expression};

pub(crate) fn is_class_component(c: &Class<'_>) -> bool {
    let Some(super_class) = &c.super_class else {
        return false;
    };
    match super_class {
        Expression::Identifier(id) => id.name == "Component" || id.name == "PureComponent",
        Expression::StaticMemberExpression(m) => {
            matches!(&m.object, Expression::Identifier(obj) if obj.name == "React")
                && (m.property.name == "Component" || m.property.name == "PureComponent")
        }
        _ => false,
    }
}

pub(crate) fn is_component_expr(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => true,
        Expression::CallExpression(call) => {
            let name = match &call.callee {
                Expression::Identifier(id) => id.name.as_ref(),
                Expression::StaticMemberExpression(m) if matches!(&m.object, Expression::Identifier(obj) if obj.name == "React") => {
                    m.property.name.as_ref()
                }
                _ => return false,
            };
            matches!(name, "memo" | "forwardRef" | "lazy" | "dynamic")
        }
        Expression::ParenthesizedExpression(p) => is_component_expr(&p.expression),
        _ => false,
    }
}
