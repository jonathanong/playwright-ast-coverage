use crate::codebase::ts_routes::refs::normalize_template;
use crate::codebase::ts_source::{byte_offset_to_line, unwrap_ts_wrappers};
use oxc::allocator::Allocator;
use oxc::ast::ast::{Argument, Declaration, Expression, Program, Statement};
use oxc::parser::Parser;
use oxc::span::SourceType;

const HTTP_VERBS: &[&str] = &["get", "post", "put", "patch", "delete", "head", "options"];

/// A detected HTTP client call with a literal path.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpCall {
    pub path: String,
    pub line: u32,
}

/// Extract HTTP method calls from `source` whose path starts with one of `prefixes`.
///
/// Detects:
/// - `<any>.<verb>('<path>', ...)` — chained method call with static first arg
/// - `fetch('<path>', ...)` — Fetch API with static first arg
///
/// Only calls whose path starts with one of `prefixes` are returned.
/// Template literals are accepted only when they have no interpolations; other
/// dynamic paths are skipped.
pub fn extract_http_calls(source: &str, prefixes: &[&str]) -> Vec<HttpCall> {
    let allocator = Allocator::default();
    let source_type = SourceType::tsx();
    let ret = Parser::new(&allocator, source, source_type).parse();
    extract_http_calls_from_program(&ret.program, source, prefixes)
}

pub fn extract_http_calls_from_program<'a>(
    program: &Program<'a>,
    source: &str,
    prefixes: &[&str],
) -> Vec<HttpCall> {
    let mut results = Vec::new();

    for stmt in &program.body {
        collect_from_stmt(stmt, source, prefixes, &mut results);
    }

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
        }
        Statement::ForStatement(f) => {
            if let Some(oxc::ast::ast::ForStatementInit::VariableDeclaration(v)) = &f.init {
                for decl in &v.declarations {
                    if let Some(init) = &decl.init {
                        collect_from_expr(init, source, prefixes, out);
                    }
                }
            }
            collect_from_stmt(&f.body, source, prefixes, out);
        }
        Statement::ForInStatement(f) => collect_from_stmt(&f.body, source, prefixes, out),
        Statement::ForOfStatement(f) => collect_from_stmt(&f.body, source, prefixes, out),
        Statement::WhileStatement(w) => collect_from_stmt(&w.body, source, prefixes, out),
        Statement::DoWhileStatement(d) => collect_from_stmt(&d.body, source, prefixes, out),
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

            // Recurse into callee (handles chaining: a.b().get('/path'))
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
            collect_from_expr(&cond.consequent, source, prefixes, out);
            collect_from_expr(&cond.alternate, source, prefixes, out);
        }
        Expression::LogicalExpression(logical) => {
            collect_from_expr(&logical.left, source, prefixes, out);
            collect_from_expr(&logical.right, source, prefixes, out);
        }
        Expression::StaticMemberExpression(m) => {
            collect_from_expr(&m.object, source, prefixes, out);
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

#[cfg(test)]
mod tests;
