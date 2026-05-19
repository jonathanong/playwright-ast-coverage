use super::HttpCall;
use crate::codebase::ts_routes::refs::normalize_template;
use crate::codebase::ts_source::{byte_offset_to_line, unwrap_ts_wrappers};
use oxc::ast::ast::{Argument, Declaration, Expression, ForStatementInit, Program, Statement};

const HTTP_VERBS: &[&str] = &["get", "post", "put", "patch", "delete", "head", "options"];

pub(super) fn extract_http_calls_from_program<'a>(
    program: &Program<'a>,
    source: &str,
    prefixes: &[&str],
) -> Vec<HttpCall> {
    let mut results = Vec::new();
    program
        .body
        .iter()
        .for_each(|stmt| collect_from_stmt(stmt, source, prefixes, &mut results));
    results
}

fn collect_from_stmt(stmt: &Statement, source: &str, prefixes: &[&str], out: &mut Vec<HttpCall>) {
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
        Statement::ForStatement(f) => {
            if let Some(init) = &f.init {
                match init {
                    ForStatementInit::VariableDeclaration(v) => {
                        for decl in &v.declarations {
                            if let Some(init) = &decl.init {
                                collect_from_expr(init, source, prefixes, out);
                            }
                        }
                    }
                    other => {
                        if let Some(expr) = other.as_expression() {
                            collect_from_expr(expr, source, prefixes, out);
                        }
                    }
                }
            }
            if let Some(test) = &f.test {
                collect_from_expr(test, source, prefixes, out);
            }
            if let Some(update) = &f.update {
                collect_from_expr(update, source, prefixes, out);
            }
            collect_from_stmt(&f.body, source, prefixes, out);
        }
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
    kind: &oxc::ast::ast::ExportDefaultDeclarationKind,
    source: &str,
    prefixes: &[&str],
    out: &mut Vec<HttpCall>,
) {
    match kind {
        oxc::ast::ast::ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
            if let Some(body) = &f.body {
                for s in &body.statements {
                    collect_from_stmt(s, source, prefixes, out);
                }
            }
        }
        oxc::ast::ast::ExportDefaultDeclarationKind::ArrowFunctionExpression(a) => {
            for s in &a.body.statements {
                collect_from_stmt(s, source, prefixes, out);
            }
        }
        _ => {}
    }
}

fn collect_from_expr(expr: &Expression, source: &str, prefixes: &[&str], out: &mut Vec<HttpCall>) {
    let expr = unwrap_ts_wrappers(expr);
    match expr {
        Expression::CallExpression(call) => {
            let line = byte_offset_to_line(source, call.span.start as usize);
            let is_http_verb_call = match &call.callee {
                Expression::StaticMemberExpression(member) => {
                    HTTP_VERBS.contains(&member.property.name.as_str())
                }
                _ => false,
            };
            let is_fetch_call = matches!(
                unwrap_ts_wrappers(&call.callee),
                Expression::Identifier(id) if id.name.as_str() == "fetch"
            );
            if is_http_verb_call || is_fetch_call {
                if let Some(path) = static_path_arg(&call.arguments, 0) {
                    if prefixes.iter().any(|p| path.starts_with(*p)) {
                        out.push(HttpCall { path, line });
                    }
                }
            }
            collect_from_expr(&call.callee, source, prefixes, out);
            for arg in &call.arguments {
                if let Some(e) = arg.as_expression() {
                    collect_from_expr(e, source, prefixes, out);
                }
            }
        }
        Expression::AwaitExpression(a) => collect_from_expr(&a.argument, source, prefixes, out),
        Expression::ArrowFunctionExpression(arrow) => {
            for s in &arrow.body.statements {
                collect_from_stmt(s, source, prefixes, out);
            }
        }
        Expression::ConditionalExpression(cond) => {
            collect_from_expr(&cond.test, source, prefixes, out);
            collect_from_expr(&cond.consequent, source, prefixes, out);
            collect_from_expr(&cond.alternate, source, prefixes, out);
        }
        Expression::LogicalExpression(logical) => {
            collect_from_expr(&logical.left, source, prefixes, out);
            collect_from_expr(&logical.right, source, prefixes, out);
        }
        Expression::StaticMemberExpression(m) => {
            collect_from_expr(&m.object, source, prefixes, out)
        }
        Expression::SequenceExpression(s) => {
            for e in &s.expressions {
                collect_from_expr(e, source, prefixes, out);
            }
        }
        _ => {}
    }
}

fn static_path_arg(args: &[Argument], index: usize) -> Option<String> {
    match args.get(index)? {
        Argument::StringLiteral(s) => Some(s.value.as_str().to_string()),
        Argument::TemplateLiteral(tl) if tl.expressions.is_empty() => Some(normalize_template(tl)),
        _ => None,
    }
}
