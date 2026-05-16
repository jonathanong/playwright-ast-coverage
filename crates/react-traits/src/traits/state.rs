use oxc_ast::ast::{Expression, Program};
use oxc_ast_visit::{walk, Visit};

struct StateVisitor {
    has_state: bool,
}

impl<'a> Visit<'a> for StateVisitor {
    fn visit_call_expression(&mut self, expr: &oxc_ast::ast::CallExpression<'a>) {
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
        if matches!(&expr.object, Expression::ThisExpression(_)) {
            let prop = expr.property.name.as_ref();
            if prop == "state" || prop == "setState" {
                self.has_state = true;
            }
        }
        walk::walk_static_member_expression(self, expr);
    }
}

pub(crate) fn detect_has_state(program: &Program<'_>, _source: &str) -> bool {
    let mut visitor = StateVisitor { has_state: false };
    visitor.visit_program(program);
    visitor.has_state
}

#[cfg(test)]
mod tests;
