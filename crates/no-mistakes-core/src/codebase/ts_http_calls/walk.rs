use super::HttpCall;
use crate::codebase::ts_routes::refs::normalize_template;
use crate::codebase::ts_source::{byte_offset_to_line, unwrap_ts_wrappers};
use oxc::ast::ast::{Argument, Expression, Program};

mod assignment_target;
mod stmt;

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
        .for_each(|stmt| stmt::collect_from_stmt(stmt, source, prefixes, &mut results));
    results
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
                stmt::collect_from_stmt(s, source, prefixes, out);
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
