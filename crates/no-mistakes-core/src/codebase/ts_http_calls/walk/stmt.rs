use super::{assignment_target::collect_from_assignment_target, collect_from_expr, HttpCall};
use oxc::ast::ast::{Declaration, ExportDefaultDeclarationKind, ForStatementInit, Statement};

pub(super) fn collect_from_stmt(
    stmt: &Statement,
    source: &str,
    prefixes: &[&str],
    out: &mut Vec<HttpCall>,
) {
    match stmt {
        Statement::ExpressionStatement(s) => {
            collect_from_expr(&s.expression, source, prefixes, out)
        }
        Statement::ReturnStatement(r) => {
            if let Some(e) = &r.argument {
                collect_from_expr(e, source, prefixes, out);
            }
        }
        Statement::VariableDeclaration(v) => {
            for decl in &v.declarations {
                if let Some(init) = &decl.init {
                    collect_from_expr(init, source, prefixes, out);
                }
            }
        }
        Statement::BlockStatement(b) => {
            for s in &b.body {
                collect_from_stmt(s, source, prefixes, out);
            }
        }
        Statement::FunctionDeclaration(f) => {
            if let Some(body) = &f.body {
                for s in &body.statements {
                    collect_from_stmt(s, source, prefixes, out);
                }
            }
        }
        Statement::ExportNamedDeclaration(e) => {
            if let Some(decl) = &e.declaration {
                collect_from_decl(decl, source, prefixes, out);
            }
        }
        Statement::ExportDefaultDeclaration(e) => {
            collect_from_expr_kind(&e.declaration, source, prefixes, out);
        }
        Statement::IfStatement(i) => {
            collect_from_expr(&i.test, source, prefixes, out);
            collect_from_stmt(&i.consequent, source, prefixes, out);
            if let Some(alt) = &i.alternate {
                collect_from_stmt(alt, source, prefixes, out);
            }
        }
        Statement::TryStatement(t) => {
            for s in &t.block.body {
                collect_from_stmt(s, source, prefixes, out);
            }
            if let Some(handler) = &t.handler {
                for s in &handler.body.body {
                    collect_from_stmt(s, source, prefixes, out);
                }
            }
            if let Some(finalizer) = &t.finalizer {
                for s in &finalizer.body {
                    collect_from_stmt(s, source, prefixes, out);
                }
            }
        }
        Statement::ForStatement(f) => collect_from_for_stmt(f, source, prefixes, out),
        Statement::ForInStatement(f) => {
            collect_from_expr(&f.right, source, prefixes, out);
            collect_from_stmt(&f.body, source, prefixes, out);
        }
        Statement::ForOfStatement(f) => {
            collect_from_expr(&f.right, source, prefixes, out);
            collect_from_stmt(&f.body, source, prefixes, out);
        }
        Statement::WhileStatement(w) => {
            collect_from_expr(&w.test, source, prefixes, out);
            collect_from_stmt(&w.body, source, prefixes, out);
        }
        Statement::DoWhileStatement(d) => {
            collect_from_stmt(&d.body, source, prefixes, out);
            collect_from_expr(&d.test, source, prefixes, out);
        }
        _ => {}
    }
}

fn collect_from_for_stmt(
    stmt: &oxc::ast::ast::ForStatement,
    source: &str,
    prefixes: &[&str],
    out: &mut Vec<HttpCall>,
) {
    if let Some(init) = &stmt.init {
        match init {
            ForStatementInit::VariableDeclaration(v) => {
                for decl in &v.declarations {
                    if let Some(init) = &decl.init {
                        collect_from_expr(init, source, prefixes, out);
                    }
                }
            }
            ForStatementInit::AssignmentExpression(assign) => {
                collect_from_assignment_target(&assign.left, source, prefixes, out);
                collect_from_expr(&assign.right, source, prefixes, out);
            }
            other => {
                if let Some(expr) = other.as_expression() {
                    collect_from_expr(expr, source, prefixes, out);
                }
            }
        }
    }
    if let Some(test) = &stmt.test {
        collect_from_expr(test, source, prefixes, out);
    }
    if let Some(update) = &stmt.update {
        collect_from_expr(update, source, prefixes, out);
    }
    collect_from_stmt(&stmt.body, source, prefixes, out);
}

fn collect_from_decl(decl: &Declaration, source: &str, prefixes: &[&str], out: &mut Vec<HttpCall>) {
    match decl {
        Declaration::VariableDeclaration(v) => {
            for d in &v.declarations {
                if let Some(init) = &d.init {
                    collect_from_expr(init, source, prefixes, out);
                }
            }
        }
        Declaration::FunctionDeclaration(f) => {
            if let Some(body) = &f.body {
                for s in &body.statements {
                    collect_from_stmt(s, source, prefixes, out);
                }
            }
        }
        _ => {}
    }
}

fn collect_from_expr_kind(
    kind: &ExportDefaultDeclarationKind,
    source: &str,
    prefixes: &[&str],
    out: &mut Vec<HttpCall>,
) {
    match kind {
        ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
            if let Some(body) = &f.body {
                for s in &body.statements {
                    collect_from_stmt(s, source, prefixes, out);
                }
            }
        }
        ExportDefaultDeclarationKind::ArrowFunctionExpression(a) => {
            for s in &a.body.statements {
                collect_from_stmt(s, source, prefixes, out);
            }
        }
        _ => {}
    }
}
