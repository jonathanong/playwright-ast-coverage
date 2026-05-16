use oxc_ast::ast::{
    BindingPattern, Expression, JSXElementName, Program, Statement, VariableDeclaration,
};
use oxc_ast_visit::{walk, Visit};
use oxc_span::Span;
use std::collections::HashSet;

struct SuspenseVisitor<'a> {
    has_suspense: bool,
    span: Span,
    dynamic_names: &'a HashSet<String>,
}

fn within(node_span: Span, component_span: Span) -> bool {
    node_span.start >= component_span.start && node_span.end <= component_span.end
}

impl<'a> Visit<'a> for SuspenseVisitor<'a> {
    fn visit_jsx_opening_element(&mut self, elem: &oxc_ast::ast::JSXOpeningElement<'a>) {
        if !within(elem.span, self.span) {
            walk::walk_jsx_opening_element(self, elem);
            return;
        }
        let is_suspense = match &elem.name {
            JSXElementName::IdentifierReference(id) => {
                id.name == "Suspense" || self.dynamic_names.contains(id.name.as_ref())
            }
            JSXElementName::MemberExpression(m) => m.property.name == "Suspense",
            _ => false,
        };
        if is_suspense {
            self.has_suspense = true;
        }
        walk::walk_jsx_opening_element(self, elem);
    }
}

fn collect_dynamic_names(program: &Program<'_>) -> HashSet<String> {
    let mut names = HashSet::new();
    for stmt in &program.body {
        let Statement::VariableDeclaration(v) = stmt else {
            continue;
        };
        collect_from_var_decl(v, &mut names);
    }
    names
}

fn collect_from_var_decl(v: &VariableDeclaration<'_>, names: &mut HashSet<String>) {
    for decl in &v.declarations {
        let BindingPattern::BindingIdentifier(id) = &decl.id else {
            continue;
        };
        let Some(init) = &decl.init else {
            continue;
        };
        if is_dynamic_or_lazy_call(init) {
            names.insert(id.name.as_ref().to_string());
        }
    }
}

fn is_dynamic_or_lazy_call(expr: &Expression<'_>) -> bool {
    let Expression::CallExpression(call) = expr else {
        return false;
    };
    let name = match &call.callee {
        Expression::Identifier(id) => id.name.as_ref(),
        Expression::StaticMemberExpression(m) if matches!(&m.object, Expression::Identifier(obj) if obj.name == "React") => {
            m.property.name.as_ref()
        }
        _ => return false,
    };
    matches!(name, "dynamic" | "lazy")
}

pub(crate) fn detect_uses_suspense(program: &Program<'_>, span: Span) -> bool {
    let dynamic_names = collect_dynamic_names(program);
    let mut visitor = SuspenseVisitor {
        has_suspense: false,
        span,
        dynamic_names: &dynamic_names,
    };
    visitor.visit_program(program);
    visitor.has_suspense
}

#[cfg(test)]
mod tests;
