use oxc_ast::ast::{Expression, Program};
use oxc_ast_visit::{walk, Visit};
use oxc_span::Span;

struct StateVisitor {
    has_state: bool,
    span: Span,
}

fn within(node_span: Span, component_span: Span) -> bool {
    node_span.start >= component_span.start && node_span.end <= component_span.end
}

impl<'a> Visit<'a> for StateVisitor {
    fn visit_call_expression(&mut self, expr: &oxc_ast::ast::CallExpression<'a>) {
        if !within(expr.span, self.span) {
            return;
        }
        let name = match &expr.callee {
            Expression::Identifier(id) => Some(id.name.as_ref().to_string()),
            Expression::StaticMemberExpression(m) => {
                if matches!(&m.object, Expression::Identifier(id) if id.name == "React") {
                    Some(m.property.name.as_ref().to_string())
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(name) = name {
            if matches!(
                name.as_str(),
                "useState" | "useReducer" | "useOptimistic" | "useSyncExternalStore"
            ) {
                self.has_state = true;
            }
        }
        walk::walk_call_expression(self, expr);
    }

    fn visit_static_member_expression(&mut self, expr: &oxc_ast::ast::StaticMemberExpression<'a>) {
        if !within(expr.span, self.span) {
            return;
        }
        if matches!(&expr.object, Expression::ThisExpression(_)) {
            let prop = expr.property.name.as_ref();
            if prop == "state" || prop == "setState" {
                self.has_state = true;
            }
        }
        walk::walk_static_member_expression(self, expr);
    }
}

pub(crate) fn detect_has_state(program: &Program<'_>, span: Span) -> bool {
    let mut visitor = StateVisitor {
        has_state: false,
        span,
    };
    visitor.visit_program(program);
    visitor.has_state
}

#[cfg(test)]
mod tests;
