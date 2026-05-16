use crate::analyze::components::ComponentDef;
use oxc_ast::ast::{ExportDefaultDeclarationKind, Expression, Program, Statement};
use oxc_ast_visit::{walk, Visit};
use oxc_span::Span;

struct MemoVisitor {
    has_use_memo: bool,
    span: Span,
}

fn within(node_span: Span, component_span: Span) -> bool {
    node_span.start >= component_span.start && node_span.end <= component_span.end
}

impl<'a> Visit<'a> for MemoVisitor {
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
            if name == "useMemo" {
                self.has_use_memo = true;
            }
        }
        walk::walk_call_expression(self, expr);
    }
}

fn is_wrapped_in_memo(program: &Program<'_>, def: &ComponentDef) -> bool {
    if def.name != "default" {
        return false;
    }
    for stmt in &program.body {
        let Statement::ExportDefaultDeclaration(e) = stmt else {
            continue;
        };
        if let ExportDefaultDeclarationKind::CallExpression(call) = &e.declaration {
            let name = match &call.callee {
                Expression::Identifier(id) => id.name.as_ref(),
                Expression::StaticMemberExpression(m) if matches!(&m.object, Expression::Identifier(obj) if obj.name == "React") => {
                    m.property.name.as_ref()
                }
                _ => "",
            };
            if name == "memo" {
                return true;
            }
        }
    }
    false
}

pub(crate) fn detect_uses_memo(program: &Program<'_>, span: Span, def: &ComponentDef) -> bool {
    let mut visitor = MemoVisitor {
        has_use_memo: false,
        span,
    };
    visitor.visit_program(program);
    visitor.has_use_memo || is_wrapped_in_memo(program, def)
}

#[cfg(test)]
mod tests;
