use oxc_ast::ast::{
    BindingPattern, Declaration, ExportDefaultDeclarationKind, Expression, JSXElementName, Program,
    Statement, VariableDeclaration,
};
use oxc_ast_visit::{walk, Visit};
use oxc_span::Span;
use std::collections::HashSet;

struct DynamicNameCollector {
    names: HashSet<String>,
}

impl<'a> Visit<'a> for DynamicNameCollector {
    fn visit_variable_declaration(&mut self, v: &VariableDeclaration<'a>) {
        collect_from_var_decl(v, &mut self.names);
        walk::walk_variable_declaration(self, v);
    }
}

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

fn overlaps(a: Span, b: Span) -> bool {
    a.start < b.end && a.end > b.start
}

fn collect_dynamic_names(program: &Program<'_>) -> HashSet<String> {
    let mut collector = DynamicNameCollector {
        names: HashSet::new(),
    };
    collector.visit_program(program);
    collector.names
}

fn is_component_direct_lazy(program: &Program<'_>, span: Span) -> bool {
    for stmt in &program.body {
        match stmt {
            Statement::ExportDefaultDeclaration(e) if overlaps(e.span, span) => {
                if let ExportDefaultDeclarationKind::CallExpression(call) = &e.declaration {
                    if is_dynamic_or_lazy_call_by_callee(&call.callee) {
                        return true;
                    }
                }
            }
            Statement::ExportNamedDeclaration(e) => {
                if let Some(Declaration::VariableDeclaration(v)) = &e.declaration {
                    for d in &v.declarations {
                        if overlaps(d.span, span) {
                            if let Some(init) = &d.init {
                                if is_dynamic_or_lazy_call(init) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
            Statement::VariableDeclaration(v) => {
                for d in &v.declarations {
                    if overlaps(d.span, span) {
                        if let Some(init) = &d.init {
                            if is_dynamic_or_lazy_call(init) {
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

fn is_dynamic_or_lazy_call_by_callee(callee: &Expression<'_>) -> bool {
    let name = match callee {
        Expression::Identifier(id) => id.name.as_ref(),
        Expression::StaticMemberExpression(m) if matches!(&m.object, Expression::Identifier(obj) if obj.name == "React") => {
            m.property.name.as_ref()
        }
        _ => return false,
    };
    matches!(name, "dynamic" | "lazy")
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
    if is_component_direct_lazy(program, span) {
        return true;
    }
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
