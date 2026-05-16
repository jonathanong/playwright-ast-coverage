use oxc_ast::ast::{
    Declaration, ExportDefaultDeclarationKind, Expression, JSXAttributeItem, JSXElementName,
    Program, Statement,
};
use oxc_ast_visit::{walk, Visit};
use oxc_span::Span;

struct PropsVisitor {
    passes_props: bool,
    span: Span,
}

fn within(node_span: Span, component_span: Span) -> bool {
    node_span.start >= component_span.start && node_span.end <= component_span.end
}

fn overlaps(a: Span, b: Span) -> bool {
    a.start < b.end && a.end > b.start
}

impl<'a> Visit<'a> for PropsVisitor {
    fn visit_jsx_opening_element(&mut self, elem: &oxc_ast::ast::JSXOpeningElement<'a>) {
        if !within(elem.span, self.span) {
            return;
        }
        let is_component = match &elem.name {
            JSXElementName::IdentifierReference(id) => {
                id.name.chars().next().is_some_and(|c| c.is_uppercase())
            }
            JSXElementName::MemberExpression(_) => true,
            _ => false,
        };
        if is_component && !elem.attributes.is_empty() {
            for attr in &elem.attributes {
                match attr {
                    JSXAttributeItem::Attribute(_) | JSXAttributeItem::SpreadAttribute(_) => {
                        self.passes_props = true;
                    }
                }
            }
        }
        walk::walk_jsx_opening_element(self, elem);
    }
}

fn fn_has_params(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::ArrowFunctionExpression(a) => !a.params.items.is_empty(),
        Expression::FunctionExpression(f) => !f.params.items.is_empty(),
        _ => false,
    }
}

fn expr_or_wrapped_has_params(init: &Option<Expression<'_>>) -> bool {
    let Some(expr) = init else { return false };
    if fn_has_params(expr) {
        return true;
    }
    if let Expression::CallExpression(call) = expr {
        if let Some(first_arg) = call.arguments.first() {
            if let Some(inner) = first_arg.as_expression() {
                return fn_has_params(inner);
            }
        }
    }
    false
}

fn has_function_params(program: &Program<'_>, span: Span) -> bool {
    for stmt in &program.body {
        match stmt {
            Statement::ExportDefaultDeclaration(e) => match &e.declaration {
                ExportDefaultDeclarationKind::FunctionDeclaration(f)
                    if !f.params.items.is_empty() && overlaps(f.span, span) =>
                {
                    return true;
                }
                ExportDefaultDeclarationKind::ArrowFunctionExpression(a)
                    if !a.params.items.is_empty() && overlaps(e.span, span) =>
                {
                    return true;
                }
                ExportDefaultDeclarationKind::CallExpression(call) if overlaps(e.span, span) => {
                    if let Some(first_arg) = call.arguments.first() {
                        if let Some(expr) = first_arg.as_expression() {
                            match expr {
                                Expression::FunctionExpression(f) if !f.params.items.is_empty() => {
                                    return true;
                                }
                                Expression::ArrowFunctionExpression(a)
                                    if !a.params.items.is_empty() =>
                                {
                                    return true;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            },
            Statement::ExportNamedDeclaration(e) => {
                if let Some(decl) = &e.declaration {
                    match decl {
                        Declaration::FunctionDeclaration(f)
                            if !f.params.items.is_empty() && overlaps(f.span, span) =>
                        {
                            return true;
                        }
                        Declaration::VariableDeclaration(v) => {
                            for d in &v.declarations {
                                if !overlaps(d.span, span) {
                                    continue;
                                }
                                if expr_or_wrapped_has_params(&d.init) {
                                    return true;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            // Non-exported top-level decls whose span was used as the component span
            // (e.g. `const Page = (props) => ...; export default Page;`)
            Statement::VariableDeclaration(v) => {
                for d in &v.declarations {
                    if !overlaps(d.span, span) {
                        continue;
                    }
                    if expr_or_wrapped_has_params(&d.init) {
                        return true;
                    }
                }
            }
            Statement::FunctionDeclaration(f)
                if !f.params.items.is_empty() && overlaps(f.span, span) =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}

pub(crate) fn detect_props(program: &Program<'_>, span: Span) -> (bool, bool) {
    let has_props = has_function_params(program, span);
    let mut visitor = PropsVisitor {
        passes_props: false,
        span,
    };
    visitor.visit_program(program);
    (has_props, visitor.passes_props)
}

#[cfg(test)]
mod tests;
