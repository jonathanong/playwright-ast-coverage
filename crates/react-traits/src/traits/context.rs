use oxc_ast::ast::{JSXElementName, Program};
use oxc_ast_visit::{walk, Visit};

struct ContextVisitor {
    has_provider: bool,
}

impl<'a> Visit<'a> for ContextVisitor {
    fn visit_jsx_opening_element(&mut self, elem: &oxc_ast::ast::JSXOpeningElement<'a>) {
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

pub(crate) fn detect_context_provider(program: &Program<'_>, _source: &str) -> bool {
    let mut visitor = ContextVisitor {
        has_provider: false,
    };
    visitor.visit_program(program);
    visitor.has_provider
}

#[cfg(test)]
mod tests;
