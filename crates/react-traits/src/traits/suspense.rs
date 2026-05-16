use oxc_ast::ast::{
    BindingPattern, Declaration, ExportDefaultDeclarationKind, Expression, Function,
    JSXElementName, JSXMemberExpressionObject, Program, Statement, VariableDeclaration,
};
use oxc_ast_visit::{walk, Visit};
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;
use std::collections::HashSet;

struct DynamicNameCollector {
    component_span: Span,
    inner_dynamic: HashSet<String>,
    outer_dynamic: HashSet<String>,
    inner_non_dynamic: HashSet<String>,
}

impl<'a> Visit<'a> for DynamicNameCollector {
    fn visit_variable_declaration(&mut self, v: &VariableDeclaration<'a>) {
        let in_component = within(v.span, self.component_span);
        collect_from_var_decl(v, in_component, self);
        walk::walk_variable_declaration(self, v);
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        // Track every BindingIdentifier within the component span as a potential
        // shadow of an outer dynamic name (covers function params, destructuring, etc.).
        if let BindingPattern::BindingIdentifier(id) = it {
            if within(id.span, self.component_span) {
                self.inner_non_dynamic.insert(id.name.as_ref().to_string());
            }
        }
        walk::walk_binding_pattern(self, it);
    }

    fn visit_function(&mut self, func: &Function<'a>, flags: ScopeFlags) {
        // A function declaration name (e.g. `function Lazy() {}`) inside the component
        // span shadows any outer `const Lazy = dynamic(...)` binding.
        if within(func.span, self.component_span) {
            if let Some(id) = &func.id {
                self.inner_non_dynamic.insert(id.name.as_ref().to_string());
            }
        }
        walk::walk_function(self, func, flags);
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
            JSXElementName::MemberExpression(m) => {
                m.property.name == "Suspense"
                    && matches!(
                        &m.object,
                        JSXMemberExpressionObject::IdentifierReference(obj) if obj.name == "React"
                    )
            }
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

fn collect_dynamic_names(program: &Program<'_>, component_span: Span) -> HashSet<String> {
    let mut collector = DynamicNameCollector {
        component_span,
        inner_dynamic: HashSet::new(),
        outer_dynamic: HashSet::new(),
        inner_non_dynamic: HashSet::new(),
    };
    collector.visit_program(program);

    // Effective dynamic names = inner_dynamic ∪ (outer_dynamic ∖ inner_non_dynamic).
    // Declarations inside the component body shadow outer-scope bindings of the same name;
    // if the inner binding is non-dynamic, the outer dynamic one is no longer reachable.
    let mut names = collector.inner_dynamic;
    for name in collector.outer_dynamic {
        if !collector.inner_non_dynamic.contains(&name) {
            names.insert(name);
        }
    }
    names
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
    match callee {
        Expression::Identifier(id) => matches!(id.name.as_ref(), "dynamic" | "lazy"),
        Expression::StaticMemberExpression(m) if matches!(&m.object, Expression::Identifier(obj) if obj.name == "React") => {
            m.property.name.as_ref() == "lazy"
        }
        _ => false,
    }
}

fn collect_from_var_decl(
    v: &VariableDeclaration<'_>,
    in_component: bool,
    collector: &mut DynamicNameCollector,
) {
    for decl in &v.declarations {
        let BindingPattern::BindingIdentifier(id) = &decl.id else {
            continue;
        };
        let Some(init) = &decl.init else {
            continue;
        };
        let name = id.name.as_ref().to_string();
        if is_dynamic_or_lazy_call(init) {
            if in_component {
                collector.inner_dynamic.insert(name);
            } else {
                collector.outer_dynamic.insert(name);
            }
        } else if in_component {
            collector.inner_non_dynamic.insert(name);
        }
    }
}

fn is_dynamic_or_lazy_call(expr: &Expression<'_>) -> bool {
    let Expression::CallExpression(call) = expr else {
        return false;
    };
    match &call.callee {
        Expression::Identifier(id) => matches!(id.name.as_ref(), "dynamic" | "lazy"),
        Expression::StaticMemberExpression(m) if matches!(&m.object, Expression::Identifier(obj) if obj.name == "React") => {
            m.property.name.as_ref() == "lazy"
        }
        _ => false,
    }
}

pub(crate) fn detect_uses_suspense(program: &Program<'_>, span: Span) -> bool {
    if is_component_direct_lazy(program, span) {
        return true;
    }
    let dynamic_names = collect_dynamic_names(program, span);
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
