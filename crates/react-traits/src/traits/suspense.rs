use oxc_ast::ast::{JSXElementName, Program, Statement};
use oxc_ast_visit::{walk, Visit};

struct SuspenseVisitor {
    has_suspense: bool,
}

impl<'a> Visit<'a> for SuspenseVisitor {
    fn visit_jsx_opening_element(&mut self, elem: &oxc_ast::ast::JSXOpeningElement<'a>) {
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

pub(crate) fn detect_uses_suspense(program: &Program<'_>, _source: &str) -> bool {
    let mut visitor = SuspenseVisitor {
        has_suspense: false,
    };
    visitor.visit_program(program);
    visitor.has_suspense || has_dynamic_import(program)
}

#[cfg(test)]
mod tests;
