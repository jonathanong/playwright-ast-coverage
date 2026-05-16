use oxc_ast::ast::{JSXElementName, Program, Statement};
use oxc_ast_visit::{walk, Visit};
use oxc_span::Span;

struct SuspenseVisitor {
    has_suspense: bool,
    span: Span,
}

fn within(node_span: Span, component_span: Span) -> bool {
    node_span.start >= component_span.start && node_span.end <= component_span.end
}

impl<'a> Visit<'a> for SuspenseVisitor {
    fn visit_jsx_opening_element(&mut self, elem: &oxc_ast::ast::JSXOpeningElement<'a>) {
        if !within(elem.span, self.span) {
            return;
        }
        let is_suspense = match &elem.name {
            JSXElementName::IdentifierReference(id) => id.name == "Suspense",
            JSXElementName::MemberExpression(m) => m.property.name == "Suspense",
            _ => false,
        };
        if is_suspense {
            self.has_suspense = true;
        }
        walk::walk_jsx_opening_element(self, elem);
    }
}

fn has_dynamic_import(program: &Program<'_>) -> bool {
    for stmt in &program.body {
        let Statement::ImportDeclaration(import) = stmt else {
            continue;
        };
        if import.source.value == "next/dynamic" {
            return true;
        }
    }
    false
}

pub(crate) fn detect_uses_suspense(program: &Program<'_>, span: Span) -> bool {
    let mut visitor = SuspenseVisitor {
        has_suspense: false,
        span,
    };
    visitor.visit_program(program);
    visitor.has_suspense || has_dynamic_import(program)
}

#[cfg(test)]
mod tests;
