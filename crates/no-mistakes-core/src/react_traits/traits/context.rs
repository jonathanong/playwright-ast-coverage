use oxc_ast::ast::{JSXElementName, Program};
use oxc_ast_visit::{walk, Visit};
use oxc_span::Span;

struct ContextVisitor {
    has_provider: bool,
    span: Span,
}

fn within(node_span: Span, component_span: Span) -> bool {
    node_span.start >= component_span.start && node_span.end <= component_span.end
}

impl<'a> Visit<'a> for ContextVisitor {
    fn visit_jsx_opening_element(&mut self, elem: &oxc_ast::ast::JSXOpeningElement<'a>) {
        if !within(elem.span, self.span) {
            return;
        }
        match &elem.name {
            JSXElementName::MemberExpression(m) if m.property.name == "Provider" => {
                self.has_provider = true;
            }
            JSXElementName::IdentifierReference(id) if id.name == "Provider" => {
                self.has_provider = true;
            }
            _ => {}
        }
        walk::walk_jsx_opening_element(self, elem);
    }
}

pub(crate) fn detect_context_provider(program: &Program<'_>, span: Span) -> bool {
    let mut visitor = ContextVisitor {
        has_provider: false,
        span,
    };
    visitor.visit_program(program);
    visitor.has_provider
}

#[cfg(test)]
mod tests;
