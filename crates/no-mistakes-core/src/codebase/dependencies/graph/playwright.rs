use crate::codebase::ts_routes::refs::normalize_template;
use oxc::allocator::Allocator;
use oxc::ast::ast::{Argument, Expression, ForStatementInit, FunctionBody, Statement};
use oxc::parser::Parser;
use oxc::span::SourceType;

/// Extract URL string literals navigated to in a Playwright test file.
///
/// Recognises:
/// - `page.goto('<url>')`
/// - ``page.goto(`/users/${id}`)`` (normalizes interpolations to `:param`)
/// - `page.click('a[href="<url>"]')` / `page.click("a[href='<url>']")`
/// - `navigateTo(page, '<url>')` / `navigateTo('<url>')`
/// - `expect(page).toHaveURL('<url>')` and `page.waitForURL('<url>')`
pub fn extract_playwright_urls(source: &str) -> Vec<String> {
    let allocator = Allocator::default();
    let source_type = SourceType::tsx();
    let ret = Parser::new(&allocator, source, source_type).parse();

    let mut urls = Vec::new();

    for stmt in &ret.program.body {
        collect_urls_from_stmt(stmt, &mut urls);
    }

    urls.sort();
    urls.dedup();
    urls
}

fn collect_urls_from_stmt(stmt: &Statement, urls: &mut Vec<String>) {
    match stmt {
        Statement::ExpressionStatement(s) => collect_urls_from_expr(&s.expression, urls),
        Statement::VariableDeclaration(v) => {
            for decl in &v.declarations {
                if let Some(init) = &decl.init {
                    collect_urls_from_expr(init, urls);
                }
            }
        }
        Statement::ReturnStatement(r) => {
            if let Some(e) = &r.argument {
                collect_urls_from_expr(e, urls);
            }
        }
        Statement::BlockStatement(b) => {
            for s in &b.body {
                collect_urls_from_stmt(s, urls);
            }
        }
        Statement::FunctionDeclaration(f) => {
            collect_urls_from_body(f.body.as_deref(), urls);
        }
        Statement::IfStatement(i) => {
            collect_urls_from_expr(&i.test, urls);
            collect_urls_from_stmt(&i.consequent, urls);
            if let Some(alt) = &i.alternate {
                collect_urls_from_stmt(alt, urls);
            }
        }
        Statement::TryStatement(t) => {
            collect_urls_from_stmts(&t.block.body, urls);
            if let Some(handler) = &t.handler {
                collect_urls_from_stmts(&handler.body.body, urls);
            }
            if let Some(finalizer) = &t.finalizer {
                collect_urls_from_stmts(&finalizer.body, urls);
            }
        }
        Statement::WhileStatement(w) => {
            collect_urls_from_expr(&w.test, urls);
            collect_urls_from_stmt(&w.body, urls);
        }
        Statement::DoWhileStatement(d) => {
            collect_urls_from_stmt(&d.body, urls);
            collect_urls_from_expr(&d.test, urls);
        }
        Statement::ForStatement(f) => {
            if let Some(init) = &f.init {
                match init {
                    ForStatementInit::VariableDeclaration(v) => {
                        for decl in &v.declarations {
                            if let Some(init) = &decl.init {
                                collect_urls_from_expr(init, urls);
                            }
                        }
                    }
                    other => {
                        if let Some(expr) = other.as_expression() {
                            collect_urls_from_expr(expr, urls);
                        }
                    }
                }
            }
            if let Some(test) = &f.test {
                collect_urls_from_expr(test, urls);
            }
            if let Some(update) = &f.update {
                collect_urls_from_expr(update, urls);
            }
            collect_urls_from_stmt(&f.body, urls);
        }
        Statement::ForInStatement(f) => {
            collect_urls_from_expr(&f.right, urls);
            collect_urls_from_stmt(&f.body, urls);
        }
        Statement::ForOfStatement(f) => {
            collect_urls_from_expr(&f.right, urls);
            collect_urls_from_stmt(&f.body, urls);
        }
        Statement::SwitchStatement(s) => {
            collect_urls_from_expr(&s.discriminant, urls);
            for case in &s.cases {
                if let Some(test) = &case.test {
                    collect_urls_from_expr(test, urls);
                }
                for stmt in &case.consequent {
                    collect_urls_from_stmt(stmt, urls);
                }
            }
        }
        _ => {}
    }
}

fn collect_urls_from_expr(expr: &Expression, urls: &mut Vec<String>) {
    match expr {
        Expression::CallExpression(call) => {
            if let Expression::StaticMemberExpression(member) = &call.callee {
                let method = member.property.name.as_str();
                if method == "goto" {
                    if is_page_receiver(&member.object) {
                        if let Some(url) = route_arg(&call.arguments, 0) {
                            if url.starts_with('/') {
                                urls.push(url);
                            }
                        }
                    }
                } else if method == "click" {
                    if is_page_receiver(&member.object) {
                        if let Some(url) = route_arg(&call.arguments, 0)
                            .as_deref()
                            .and_then(extract_href_from_selector)
                        {
                            urls.push(url);
                        }
                    }
                } else if method == "waitForURL" && is_page_receiver(&member.object) {
                    if let Some(url) =
                        route_arg(&call.arguments, 0).filter(|url| url.starts_with('/'))
                    {
                        urls.push(url);
                    }
                } else if method == "toHaveURL" && is_expect_page_call(&member.object) {
                    if let Some(url) =
                        route_arg(&call.arguments, 0).filter(|url| url.starts_with('/'))
                    {
                        urls.push(url);
                    }
                }
            } else if matches!(&call.callee, Expression::Identifier(callee) if callee.name == "navigateTo")
            {
                for index in [0, 1] {
                    if let Some(url) =
                        route_arg(&call.arguments, index).filter(|url| url.starts_with('/'))
                    {
                        urls.push(url);
                        break;
                    }
                }
            }
            // Recurse into arguments.
            for arg in &call.arguments {
                if let Some(e) = arg.as_expression() {
                    collect_urls_from_expr(e, urls);
                }
            }
        }
        Expression::AwaitExpression(a) => collect_urls_from_expr(&a.argument, urls),
        Expression::ArrowFunctionExpression(arrow) => {
            collect_urls_from_stmts(&arrow.body.statements, urls);
        }
        Expression::ConditionalExpression(c) => {
            collect_urls_from_expr(&c.test, urls);
            collect_urls_from_expr(&c.consequent, urls);
            collect_urls_from_expr(&c.alternate, urls);
        }
        Expression::LogicalExpression(l) => {
            collect_urls_from_expr(&l.left, urls);
            collect_urls_from_expr(&l.right, urls);
        }
        Expression::SequenceExpression(s) => {
            for expr in &s.expressions {
                collect_urls_from_expr(expr, urls);
            }
        }
        _ => {}
    }
}

fn collect_urls_from_body(body: Option<&FunctionBody>, urls: &mut Vec<String>) {
    if let Some(body) = body {
        collect_urls_from_stmts(&body.statements, urls);
    }
}

fn collect_urls_from_stmts(statements: &[Statement], urls: &mut Vec<String>) {
    for stmt in statements {
        collect_urls_from_stmt(stmt, urls);
    }
}

fn is_page_receiver(expr: &Expression) -> bool {
    matches!(expr, Expression::Identifier(id) if id.name.as_str() == "page")
}

fn is_expect_page_call(expr: &Expression) -> bool {
    let Expression::CallExpression(call) = expr else {
        return false;
    };
    let Expression::Identifier(callee) = &call.callee else {
        return false;
    };
    if callee.name.as_str() != "expect" {
        return false;
    }
    matches!(
        call.arguments.first().and_then(|arg| arg.as_expression()),
        Some(Expression::Identifier(id)) if id.name.as_str() == "page"
    )
}

fn route_arg(args: &[Argument], index: usize) -> Option<String> {
    let arg = args.get(index)?;
    match arg {
        Argument::StringLiteral(s) => Some(s.value.as_str().to_string()),
        Argument::TemplateLiteral(tpl) => Some(normalize_template(tpl)),
        _ => arg.as_expression().and_then(route_expr),
    }
}

fn route_expr(expr: &Expression) -> Option<String> {
    match expr {
        Expression::StringLiteral(s) => Some(s.value.as_str().to_string()),
        Expression::TemplateLiteral(tpl) => Some(normalize_template(tpl)),
        Expression::ParenthesizedExpression(paren) => route_expr(&paren.expression),
        Expression::NewExpression(new_expr) => {
            let Expression::Identifier(callee) = &new_expr.callee else {
                return None;
            };
            if callee.name.as_str() != "RegExp" {
                return None;
            }
            route_arg(&new_expr.arguments, 0)
        }
        _ => None,
    }
}

/// Parse `a[href="/users/42"]` or `a[href='/users/42']` → `/users/42`.
fn extract_href_from_selector(selector: &str) -> Option<String> {
    let start = selector.find("[href=")?;
    let rest = &selector[start + 6..];
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &rest[quote.len_utf8()..];
    let end = rest.find(quote)?;
    let url = &rest[..end];
    if url.starts_with('/') {
        Some(url.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
