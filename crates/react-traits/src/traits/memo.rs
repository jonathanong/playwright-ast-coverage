use crate::analyze::components::ComponentDef;
use oxc_ast::ast::{ExportDefaultDeclarationKind, Expression, Program, Statement};
use oxc_ast_visit::{walk, Visit};

struct MemoVisitor {
    has_use_memo: bool,
}

impl<'a> Visit<'a> for MemoVisitor {
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
            if name == "useMemo" {
                self.has_use_memo = true;
            }
        }
        walk::walk_call_expression(self, expr);
    }
}

fn is_wrapped_in_memo(program: &Program<'_>, _def: &ComponentDef) -> bool {
    for stmt in &program.body {
        let Statement::ExportDefaultDeclaration(e) = stmt else {
            continue;
        };
        if let ExportDefaultDeclarationKind::CallExpression(call) = &e.declaration {
            let name = match &call.callee {
                Expression::Identifier(id) => id.name.as_ref(),
                Expression::StaticMemberExpression(m) => m.property.name.as_ref(),
                _ => "",
            };
            if matches!(name, "memo" | "forwardRef") {
                return true;
            }
        }
    }
    false
}

pub(crate) fn detect_uses_memo(program: &Program<'_>, _source: &str, def: &ComponentDef) -> bool {
    let mut visitor = MemoVisitor {
        has_use_memo: false,
    };
    visitor.visit_program(program);
    visitor.has_use_memo || is_wrapped_in_memo(program, def)
}

#[cfg(test)]
mod tests;
