//! Shared AST traversal helpers for guardrails rules that inspect JSX and/or
//! expressions inside TSX sources.
//!
//! Each helper walks the program once via a `Visitor` trait. Rules implement
//! the hooks they care about and leave the rest as no-ops.

use oxc::ast::ast::{
    Argument, ArrayExpressionElement, ClassElement, Declaration, ExportDefaultDeclarationKind,
    Expression, ForStatementInit, ImportDeclaration, JSXAttributeItem, JSXAttributeValue, JSXChild,
    JSXElement, JSXExpression, JSXOpeningElement, ObjectPropertyKind, Program, Statement,
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

fn walk_statement<V: Visitor>(stmt: &Statement, v: &mut V) {
    match stmt {
        Statement::ImportDeclaration(import) => {
            v.visit_import(import);
        }
        Statement::ExpressionStatement(e) => walk_expression(&e.expression, v),
        Statement::ReturnStatement(r) => {
            if let Some(arg) = &r.argument {
                walk_expression(arg, v);
            }
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
            if let Some(alt) = &i.alternate {
                walk_statement(alt, v);
            }
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
                            if let Some(i) = &d.init {
                                walk_expression(i, v);
                            }
                        }
                    }
                    other => {
                        if let Some(expr) = other.as_expression() {
                            walk_expression(expr, v);
                        }
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
                if let Some(init) = &d.init {
                    walk_expression(init, v);
                }
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
            if let Some(body) = &f.body {
                for s in &body.statements {
                    walk_statement(s, v);
                }
            }
        }
        Statement::ClassDeclaration(class) => {
            for item in &class.body.body {
                if let ClassElement::MethodDefinition(m) = item {
                    if let Some(body) = &m.value.body {
                        for s in &body.statements {
                            walk_statement(s, v);
                        }
                    }
                }
            }
        }
        Statement::ExportNamedDeclaration(e) => {
            if let Some(decl) = &e.declaration {
                walk_declaration(decl, v);
            }
        }
        Statement::ExportDefaultDeclaration(e) => match &e.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                if let Some(body) = &f.body {
                    for s in &body.statements {
                        walk_statement(s, v);
                    }
                }
            }
            ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                for item in &class.body.body {
                    if let ClassElement::MethodDefinition(m) = item {
                        if let Some(body) = &m.value.body {
                            for s in &body.statements {
                                walk_statement(s, v);
                            }
                        }
                    }
                }
            }
            other => {
                if let Some(expr) = other.as_expression() {
                    walk_expression(expr, v);
                }
            }
        },
        _ => {}
    }
}

fn walk_declaration<V: Visitor>(decl: &Declaration, v: &mut V) {
    match decl {
        Declaration::VariableDeclaration(var_decl) => {
            for d in &var_decl.declarations {
                if let Some(init) = &d.init {
                    walk_expression(init, v);
                }
            }
        }
        Declaration::FunctionDeclaration(f) => {
            if let Some(body) = &f.body {
                for s in &body.statements {
                    walk_statement(s, v);
                }
            }
        }
        Declaration::ClassDeclaration(class) => {
            for item in &class.body.body {
                if let ClassElement::MethodDefinition(m) = item {
                    if let Some(body) = &m.value.body {
                        for s in &body.statements {
                            walk_statement(s, v);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn walk_expression<V: Visitor>(expr: &Expression, v: &mut V) {
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
                if let Some(member) = other.as_member_expression() {
                    walk_expression(member.object(), v);
                    if let oxc::ast::ast::MemberExpression::ComputedMemberExpression(cm) = member {
                        walk_expression(&cm.expression, v);
                    }
                }
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
            if let Some(member) = a.left.as_member_expression() {
                walk_expression(member.object(), v);
                if let oxc::ast::ast::MemberExpression::ComputedMemberExpression(cm) = member {
                    walk_expression(&cm.expression, v);
                }
            }
            walk_expression(&a.right, v);
        }
        Expression::ArrowFunctionExpression(a) => {
            for s in &a.body.statements {
                walk_statement(s, v);
            }
        }
        Expression::FunctionExpression(f) => {
            if let Some(body) = &f.body {
                for s in &body.statements {
                    walk_statement(s, v);
                }
            }
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
            if let Some(member) = u.argument.as_member_expression() {
                walk_expression(member.object(), v);
                if let oxc::ast::ast::MemberExpression::ComputedMemberExpression(cm) = member {
                    walk_expression(&cm.expression, v);
                }
            }
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

fn walk_argument<V: Visitor>(arg: &Argument, v: &mut V) {
    match arg {
        Argument::SpreadElement(s) => walk_expression(&s.argument, v),
        _ => {
            if let Some(expr) = arg.as_expression() {
                walk_expression(expr, v);
            }
        }
    }
}

fn walk_jsx_element<V: Visitor>(elem: &JSXElement, v: &mut V) {
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

fn walk_jsx_child<V: Visitor>(child: &JSXChild, v: &mut V) {
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
        return Some(match &attr.value {
            None => (true, None),
            Some(JSXAttributeValue::StringLiteral(s)) => (true, Some(s.value.as_str())),
            Some(JSXAttributeValue::ExpressionContainer(c)) => {
                if let Some(expr) = c.expression.as_expression() {
                    if let Expression::StringLiteral(s) =
                        crate::codebase::ts_source::unwrap_ts_wrappers(expr)
                    {
                        (true, Some(s.value.as_str()))
                    } else {
                        (true, None)
                    }
                } else {
                    (true, None)
                }
            }
            _ => (true, None),
        });
    }
    None
}

#[cfg(test)]
mod tests;
