//! Shared AST traversal helpers for guardrails rules that inspect JSX and/or
//! expressions inside TSX sources.
//!
//! Each helper walks the program once via a `Visitor` trait. Rules implement
//! the hooks they care about and leave the rest as no-ops.

use oxc::ast::ast::{
    Argument, ArrayExpressionElement, ClassBody, ClassElement, Declaration,
    ExportDefaultDeclarationKind, Expression, ForStatementInit, FunctionBody, ImportDeclaration,
    JSXAttributeItem, JSXAttributeValue, JSXChild, JSXElement, JSXExpression, JSXOpeningElement,
    ObjectPropertyKind, Program, Statement,
};

/// Hooks rules can override. Defaults are no-ops.
pub trait Visitor {
    fn visit_import(&mut self, _import: &ImportDeclaration) {}
    fn visit_expression(&mut self, _expr: &Expression) {}
    fn visit_jsx_opening(&mut self, _opening: &JSXOpeningElement) {}
    fn visit_jsx_expression_container(&mut self, _expr: &JSXExpression, _span_start: u32) {}
}

/// Walk every statement / expression / JSX node in `program`, dispatching to
/// the visitor's hooks.
pub fn walk_program<V: Visitor>(program: &Program, v: &mut V) {
    for stmt in &program.body {
        walk_statement(stmt, v);
    }
}

fn walk_statement(stmt: &Statement, v: &mut dyn Visitor) {
    match stmt {
        Statement::ImportDeclaration(import) => {
            v.visit_import(import);
        }
        Statement::ExpressionStatement(e) => walk_expression(&e.expression, v),
        Statement::ReturnStatement(r) => {
            walk_optional_expression(r.argument.as_ref(), v);
        }
        Statement::ThrowStatement(t) => walk_expression(&t.argument, v),
        Statement::BlockStatement(b) => {
            for s in &b.body {
                walk_statement(s, v);
            }
        }
        Statement::IfStatement(i) => {
            walk_expression(&i.test, v);
            walk_statement(&i.consequent, v);
            walk_optional_statement(i.alternate.as_ref(), v);
        }
        Statement::WhileStatement(w) => {
            walk_expression(&w.test, v);
            walk_statement(&w.body, v);
        }
        Statement::DoWhileStatement(d) => {
            walk_statement(&d.body, v);
            walk_expression(&d.test, v);
        }
        Statement::ForStatement(f) => {
            if let Some(init) = &f.init {
                match init {
                    ForStatementInit::VariableDeclaration(var_decl) => {
                        for d in &var_decl.declarations {
                            walk_optional_expression(d.init.as_ref(), v);
                        }
                    }
                    other => {
                        walk_optional_expression(other.as_expression(), v);
                    }
                }
            }
            if let Some(test) = &f.test {
                walk_expression(test, v);
            }
            if let Some(update) = &f.update {
                walk_expression(update, v);
            }
            walk_statement(&f.body, v);
        }
        Statement::ForOfStatement(f) => {
            walk_expression(&f.right, v);
            walk_statement(&f.body, v);
        }
        Statement::ForInStatement(f) => {
            walk_expression(&f.right, v);
            walk_statement(&f.body, v);
        }
        Statement::VariableDeclaration(var_decl) => {
            for d in &var_decl.declarations {
                walk_optional_expression(d.init.as_ref(), v);
            }
        }
        Statement::LabeledStatement(l) => walk_statement(&l.body, v),
        Statement::TryStatement(t) => {
            for s in &t.block.body {
                walk_statement(s, v);
            }
            if let Some(handler) = &t.handler {
                for s in &handler.body.body {
                    walk_statement(s, v);
                }
            }
            if let Some(fin) = &t.finalizer {
                for s in &fin.body {
                    walk_statement(s, v);
                }
            }
        }
        Statement::SwitchStatement(s) => {
            walk_expression(&s.discriminant, v);
            for case in &s.cases {
                if let Some(test) = &case.test {
                    walk_expression(test, v);
                }
                for s in &case.consequent {
                    walk_statement(s, v);
                }
            }
        }
        Statement::FunctionDeclaration(f) => {
            walk_function_body(f.body.as_deref(), v);
        }
        Statement::ClassDeclaration(class) => walk_class_body(&class.body, v),
        Statement::ExportNamedDeclaration(e) => {
            walk_optional_declaration(e.declaration.as_ref(), v);
        }
        Statement::ExportDefaultDeclaration(e) => match &e.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                walk_function_body(f.body.as_deref(), v);
            }
            ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                walk_class_body(&class.body, v)
            }
            other => {
                walk_optional_expression(other.as_expression(), v);
            }
        },
        _ => {}
    }
}

fn walk_declaration(decl: &Declaration, v: &mut dyn Visitor) {
    match decl {
        Declaration::VariableDeclaration(var_decl) => {
            for d in &var_decl.declarations {
                walk_optional_expression(d.init.as_ref(), v);
            }
        }
        Declaration::FunctionDeclaration(f) => {
            walk_function_body(f.body.as_deref(), v);
        }
        Declaration::ClassDeclaration(class) => walk_class_body(&class.body, v),
        _ => {}
    }
}

fn walk_function_body(body: Option<&FunctionBody>, v: &mut dyn Visitor) {
    if let Some(body) = body {
        walk_statements(&body.statements, v);
    }
}

fn walk_class_body(body: &ClassBody, v: &mut dyn Visitor) {
    for item in &body.body {
        if let ClassElement::MethodDefinition(method) = item {
            walk_function_body(method.value.body.as_deref(), v);
        }
    }
}

fn walk_statements(statements: &[Statement], v: &mut dyn Visitor) {
    for statement in statements {
        walk_statement(statement, v);
    }
}

fn walk_expression(expr: &Expression, v: &mut dyn Visitor) {
    v.visit_expression(expr);
    match expr {
        Expression::CallExpression(call) => {
            walk_expression(&call.callee, v);
            for arg in &call.arguments {
                walk_argument(arg, v);
            }
        }
        Expression::NewExpression(n) => {
            walk_expression(&n.callee, v);
            for arg in &n.arguments {
                walk_argument(arg, v);
            }
        }
        Expression::ChainExpression(chain) => match &chain.expression {
            oxc::ast::ast::ChainElement::CallExpression(call) => {
                walk_expression(&call.callee, v);
                for arg in &call.arguments {
                    walk_argument(arg, v);
                }
            }
            other => {
                walk_member_expression(other.as_member_expression(), v);
            }
        },
        Expression::AwaitExpression(a) => walk_expression(&a.argument, v),
        Expression::YieldExpression(y) => {
            if let Some(arg) = &y.argument {
                walk_expression(arg, v);
            }
        }
        Expression::StaticMemberExpression(m) => walk_expression(&m.object, v),
        Expression::ComputedMemberExpression(m) => {
            walk_expression(&m.object, v);
            walk_expression(&m.expression, v);
        }
        Expression::AssignmentExpression(a) => {
            walk_member_expression(a.left.as_member_expression(), v);
            walk_expression(&a.right, v);
        }
        Expression::ArrowFunctionExpression(a) => {
            for s in &a.body.statements {
                walk_statement(s, v);
            }
        }
        Expression::FunctionExpression(f) => {
            walk_function_body(f.body.as_deref(), v);
        }
        Expression::ConditionalExpression(c) => {
            walk_expression(&c.test, v);
            walk_expression(&c.consequent, v);
            walk_expression(&c.alternate, v);
        }
        Expression::LogicalExpression(l) => {
            walk_expression(&l.left, v);
            walk_expression(&l.right, v);
        }
        Expression::BinaryExpression(b) => {
            walk_expression(&b.left, v);
            walk_expression(&b.right, v);
        }
        Expression::UnaryExpression(u) => walk_expression(&u.argument, v),
        Expression::UpdateExpression(u) => {
            walk_member_expression(u.argument.as_member_expression(), v);
        }
        Expression::SequenceExpression(s) => {
            for e in &s.expressions {
                walk_expression(e, v);
            }
        }
        Expression::ObjectExpression(o) => {
            for prop in &o.properties {
                match prop {
                    ObjectPropertyKind::ObjectProperty(p) => walk_expression(&p.value, v),
                    ObjectPropertyKind::SpreadProperty(s) => walk_expression(&s.argument, v),
                }
            }
        }
        Expression::ArrayExpression(a) => {
            for elem in &a.elements {
                if let ArrayExpressionElement::SpreadElement(s) = elem {
                    walk_expression(&s.argument, v);
                } else if let Some(e) = elem.as_expression() {
                    walk_expression(e, v);
                }
            }
        }
        Expression::ParenthesizedExpression(p) => walk_expression(&p.expression, v),
        Expression::TSAsExpression(t) => walk_expression(&t.expression, v),
        Expression::TSNonNullExpression(t) => walk_expression(&t.expression, v),
        Expression::TSSatisfiesExpression(t) => walk_expression(&t.expression, v),
        Expression::TSTypeAssertion(t) => walk_expression(&t.expression, v),
        Expression::TaggedTemplateExpression(t) => walk_expression(&t.tag, v),
        Expression::TemplateLiteral(t) => {
            for expr in &t.expressions {
                walk_expression(expr, v);
            }
        }
        Expression::JSXElement(elem) => walk_jsx_element(elem, v),
        Expression::JSXFragment(frag) => {
            for child in &frag.children {
                walk_jsx_child(child, v);
            }
        }
        _ => {}
    }
}

fn walk_member_expression(
    member: Option<&oxc::ast::ast::MemberExpression<'_>>,
    v: &mut dyn Visitor,
) {
    if let Some(member) = member {
        walk_expression(member.object(), v);
        if let oxc::ast::ast::MemberExpression::ComputedMemberExpression(cm) = member {
            walk_expression(&cm.expression, v);
        }
    }
}

fn walk_argument(arg: &Argument, v: &mut dyn Visitor) {
    match arg {
        Argument::SpreadElement(s) => walk_expression(&s.argument, v),
        _ => {
            walk_optional_expression(arg.as_expression(), v);
        }
    }
}

fn walk_optional_expression(expr: Option<&Expression>, v: &mut dyn Visitor) {
    let _ = expr.map(|expr| walk_expression(expr, v));
}

fn walk_optional_statement(stmt: Option<&Statement>, v: &mut dyn Visitor) {
    let _ = stmt.map(|stmt| walk_statement(stmt, v));
}

fn walk_optional_declaration(decl: Option<&Declaration>, v: &mut dyn Visitor) {
    let _ = decl.map(|decl| walk_declaration(decl, v));
}

fn walk_jsx_element(elem: &JSXElement, v: &mut dyn Visitor) {
    v.visit_jsx_opening(&elem.opening_element);
    for attr in &elem.opening_element.attributes {
        match attr {
            JSXAttributeItem::Attribute(a) => {
                if let Some(JSXAttributeValue::ExpressionContainer(c)) = &a.value {
                    v.visit_jsx_expression_container(&c.expression, c.span.start);
                    if let Some(expr) = c.expression.as_expression() {
                        walk_expression(expr, v);
                    }
                }
            }
            JSXAttributeItem::SpreadAttribute(s) => {
                walk_expression(&s.argument, v);
            }
        }
    }
    for child in &elem.children {
        walk_jsx_child(child, v);
    }
}

fn walk_jsx_child(child: &JSXChild, v: &mut dyn Visitor) {
    match child {
        JSXChild::Element(elem) => walk_jsx_element(elem, v),
        JSXChild::Fragment(frag) => {
            for c in &frag.children {
                walk_jsx_child(c, v);
            }
        }
        JSXChild::ExpressionContainer(container) => {
            v.visit_jsx_expression_container(&container.expression, container.span.start);
            if let Some(expr) = container.expression.as_expression() {
                walk_expression(expr, v);
            }
        }
        JSXChild::Spread(s) => walk_expression(&s.expression, v),
        _ => {}
    }
}

/// Returns true if the program contains any JSX node.
pub fn program_contains_jsx(program: &Program) -> bool {
    struct Probe(bool);
    impl Visitor for Probe {
        fn visit_expression(&mut self, expr: &Expression) {
            if matches!(expr, Expression::JSXElement(_) | Expression::JSXFragment(_)) {
                self.0 = true;
            }
        }
        fn visit_jsx_opening(&mut self, _: &JSXOpeningElement) {
            self.0 = true;
        }
    }
    let mut p = Probe(false);
    walk_program(program, &mut p);
    p.0
}

/// Returns the tag name of a JSXOpeningElement if it is a simple Identifier
/// (e.g. `<a>`, `<Link>`, `<script>`). Returns `None` for namespaced or member
/// expressions (`<foo.bar>`, `<ns:foo>`).
pub fn jsx_identifier_name<'a>(opening: &'a JSXOpeningElement) -> Option<&'a str> {
    match &opening.name {
        oxc::ast::ast::JSXElementName::Identifier(id) => Some(id.name.as_str()),
        oxc::ast::ast::JSXElementName::IdentifierReference(id) => Some(id.name.as_str()),
        _ => None,
    }
}

/// Reads a JSX attribute by name. Returns `(present, string_value_if_string_literal)`.
/// `string_value_if_string_literal` is `Some` only if the value is a string
/// literal (`x="foo"`) or an expression container wrapping a string literal
/// (`x={"foo"}`). Boolean shorthand (`<Foo bar />`) → `(true, None)`.
pub fn find_string_attr<'a>(
    opening: &'a JSXOpeningElement,
    name: &str,
) -> Option<(bool, Option<&'a str>)> {
    for item in &opening.attributes {
        let JSXAttributeItem::Attribute(attr) = item else {
            continue;
        };
        let attr_name = match &attr.name {
            oxc::ast::ast::JSXAttributeName::Identifier(id) => id.name.as_str(),
            _ => continue,
        };
        if attr_name != name {
            continue;
        }
        return Some(jsx_attr_value(&attr.value));
    }
    None
}

fn jsx_attr_value<'a>(value: &'a Option<JSXAttributeValue<'a>>) -> (bool, Option<&'a str>) {
    match value {
        None => (true, None),
        Some(JSXAttributeValue::StringLiteral(s)) => (true, Some(s.value.as_str())),
        Some(JSXAttributeValue::ExpressionContainer(c)) => match c.expression.as_expression() {
            Some(expr) => match crate::codebase::ts_source::unwrap_ts_wrappers(expr) {
                Expression::StringLiteral(s) => (true, Some(s.value.as_str())),
                _ => (true, None),
            },
            None => (true, None),
        },
        Some(_) => (true, None),
    }
}

#[cfg(test)]
mod tests;
