use crate::react_traits::analyze::components::ComponentDef;
use oxc_ast::ast::{
    BindingPattern, Declaration, ExportDefaultDeclarationKind, Expression, Program, Statement,
};
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

fn memo_callee_name<'a>(callee: &'a Expression<'_>) -> &'a str {
    match callee {
        Expression::Identifier(id) => id.name.as_ref(),
        Expression::StaticMemberExpression(m) if matches!(&m.object, Expression::Identifier(obj) if obj.name == "React") => {
            m.property.name.as_ref()
        }
        _ => "",
    }
}

fn is_wrapped_in_memo(program: &Program<'_>, def: &ComponentDef) -> bool {
    for stmt in &program.body {
        match stmt {
            Statement::ExportDefaultDeclaration(e) if def.name == "default" => {
                if let ExportDefaultDeclarationKind::CallExpression(call) = &e.declaration {
                    if memo_callee_name(&call.callee) == "memo" {
                        return true;
                    }
                }
            }
            Statement::ExportNamedDeclaration(e) if def.name != "default" => {
                if let Some(Declaration::VariableDeclaration(v)) = &e.declaration {
                    for d in &v.declarations {
                        if let BindingPattern::BindingIdentifier(id) = &d.id {
                            if id.name.as_ref() == def.name {
                                if let Some(Expression::CallExpression(call)) = &d.init {
                                    if memo_callee_name(&call.callee) == "memo" {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Handles `const Page = memo(...); export default Page;` (def.name == "default",
            // def.span covers the local declarator) and re-export alias cases like
            // `const Foo = memo(...); export { Foo as Bar };` (def.name == "Bar",
            // def.span covers Foo's declarator).
            Statement::VariableDeclaration(v) => {
                for d in &v.declarations {
                    if d.span == def.span {
                        if let Some(Expression::CallExpression(call)) = &d.init {
                            if memo_callee_name(&call.callee) == "memo" {
                                return true;
                            }
                        }
                    }
                }
            }
            _ => {}
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
