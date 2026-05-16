use crate::codebase::ts_source::byte_offset_to_line;
use oxc::allocator::Allocator;
use oxc::ast::ast::{
    Argument, ArrayExpressionElement, Expression, ImportDeclarationSpecifier, ObjectPropertyKind,
    Statement,
};
use oxc::parser::Parser;
use oxc::span::SourceType;
use std::collections::HashMap;

/// A `<binding>.add('jobName', data)` or `.addBulk([{ name: 'jobName', ... }])` call.
#[derive(Debug, Clone, PartialEq)]
pub struct EnqueueCall {
    /// Local binding identifier used (e.g. `emailsQueue`).
    pub binding: String,
    /// Job name literal if present.
    pub job: Option<String>,
    pub line: u32,
}

/// A `new Worker('queueName', handler)` declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkerDeclaration {
    /// Queue name string literal (e.g. `"emails"`), if present.
    pub queue_name: Option<String>,
    /// Import specifier of the processors namespace (`import * as processors from '...'`).
    pub processors_specifier: Option<String>,
    pub line: u32,
}

/// All queue-related usage patterns extracted from a source file.
#[derive(Debug, Default)]
pub struct QueueUsage {
    /// `(local_binding, import_specifier)` pairs for named imports.
    pub imports: Vec<(String, String)>,
    pub enqueue_calls: Vec<EnqueueCall>,
    pub worker_declarations: Vec<WorkerDeclaration>,
}

/// Scan `source` for queue usage patterns (enqueue calls and worker declarations).
///
/// Uses GlideMQ / BullMQ conventions:
/// - Enqueue: `<binding>.add('jobName', data)` or `<binding>.addBulk([{ name: 'jobName' }])`
/// - Worker:  `new Worker('queueName', handler)` where `handler` dispatches via
///   `processors[job.name]` from a namespace import `import * as processors from '...'`.
pub fn extract_queue_usage(source: &str) -> QueueUsage {
    let allocator = Allocator::default();
    let source_type = SourceType::ts();
    let ret = Parser::new(&allocator, source, source_type).parse();

    let mut usage = QueueUsage::default();

    // Pass 1: collect named imports and namespace imports.
    let mut namespace_imports: HashMap<String, String> = HashMap::new(); // local → specifier
    for stmt in &ret.program.body {
        if let Statement::ImportDeclaration(import_decl) = stmt {
            let src = import_decl.source.value.as_str();
            if let Some(specifiers) = &import_decl.specifiers {
                for spec in specifiers {
                    match spec {
                        ImportDeclarationSpecifier::ImportSpecifier(s) => {
                            let local = s.local.name.as_str().to_string();
                            usage.imports.push((local, src.to_string()));
                        }
                        ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                            namespace_imports
                                .insert(s.local.name.as_str().to_string(), src.to_string());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Also include bindings from default imports (less common for queues, but handle for robustness).
    for stmt in &ret.program.body {
        if let Statement::ImportDeclaration(import_decl) = stmt {
            let src = import_decl.source.value.as_str();
            if let Some(specifiers) = &import_decl.specifiers {
                for spec in specifiers {
                    if let ImportDeclarationSpecifier::ImportDefaultSpecifier(s) = spec {
                        let local = s.local.name.as_str().to_string();
                        usage.imports.push((local, src.to_string()));
                    }
                }
            }
        }
    }

    // Pass 2: scan statements for enqueue calls and Worker constructors.
    for stmt in &ret.program.body {
        scan_stmt(stmt, source, &namespace_imports, &mut usage);
    }

    usage
}

fn scan_stmt(
    stmt: &Statement,
    source: &str,
    namespace_imports: &HashMap<String, String>,
    usage: &mut QueueUsage,
) {
    match stmt {
        Statement::ExpressionStatement(s) => {
            scan_expr(&s.expression, source, namespace_imports, usage);
        }
        Statement::VariableDeclaration(v) => {
            for decl in &v.declarations {
                if let Some(init) = &decl.init {
                    scan_expr(init, source, namespace_imports, usage);
                }
            }
        }
        Statement::ExportNamedDeclaration(e) => {
            if let Some(decl) = &e.declaration {
                match decl {
                    oxc::ast::ast::Declaration::VariableDeclaration(v) => {
                        for d in &v.declarations {
                            if let Some(init) = &d.init {
                                scan_expr(init, source, namespace_imports, usage);
                            }
                        }
                    }
                    oxc::ast::ast::Declaration::FunctionDeclaration(f) => {
                        if let Some(body) = &f.body {
                            for s in &body.statements {
                                scan_stmt(s, source, namespace_imports, usage);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Statement::ReturnStatement(r) => {
            if let Some(expr) = &r.argument {
                scan_expr(expr, source, namespace_imports, usage);
            }
        }
        Statement::BlockStatement(b) => {
            for s in &b.body {
                scan_stmt(s, source, namespace_imports, usage);
            }
        }
        Statement::FunctionDeclaration(f) => {
            if let Some(body) = &f.body {
                for s in &body.statements {
                    scan_stmt(s, source, namespace_imports, usage);
                }
            }
        }
        Statement::IfStatement(i) => {
            scan_stmt(&i.consequent, source, namespace_imports, usage);
            if let Some(alt) = &i.alternate {
                scan_stmt(alt, source, namespace_imports, usage);
            }
        }
        Statement::TryStatement(t) => {
            for s in &t.block.body {
                scan_stmt(s, source, namespace_imports, usage);
            }
            if let Some(handler) = &t.handler {
                for s in &handler.body.body {
                    scan_stmt(s, source, namespace_imports, usage);
                }
            }
        }
        _ => {}
    }
}

fn scan_expr(
    expr: &Expression,
    source: &str,
    namespace_imports: &HashMap<String, String>,
    usage: &mut QueueUsage,
) {
    match expr {
        Expression::CallExpression(call) => {
            // Check for <binding>.add(...) or <binding>.addBulk(...)
            if let Some((binding, method)) = extract_member_call(expr) {
                if method == "add" {
                    let job = call.arguments.first().and_then(|a| literal_str(a));
                    let line = byte_offset_to_line(source, call.span.start as usize);
                    usage.enqueue_calls.push(EnqueueCall {
                        binding: binding.clone(),
                        job,
                        line,
                    });
                } else if method == "addBulk" {
                    if let Some(Argument::ArrayExpression(arr)) = call.arguments.first() {
                        for el in &arr.elements {
                            if let ArrayExpressionElement::ObjectExpression(obj) = el {
                                for prop in &obj.properties {
                                    if let ObjectPropertyKind::ObjectProperty(p) = prop {
                                        if p.key.static_name().as_deref() == Some("name") {
                                            let job = literal_str_expr(&p.value);
                                            let line = byte_offset_to_line(
                                                source,
                                                call.span.start as usize,
                                            );
                                            usage.enqueue_calls.push(EnqueueCall {
                                                binding: binding.clone(),
                                                job,
                                                line,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Check for `new Worker(...)` (handled below in NewExpression).
            // Recurse into call arguments for nested expressions.
            for arg in &call.arguments {
                if let Some(e) = arg.as_expression() {
                    scan_expr(e, source, namespace_imports, usage);
                }
            }
            // Recurse into callee.
            scan_expr(&call.callee, source, namespace_imports, usage);
        }
        Expression::NewExpression(new_expr) => {
            let callee_name = match &new_expr.callee {
                Expression::Identifier(id) => Some(id.name.as_str()),
                _ => None,
            };
            if callee_name == Some("Worker") {
                let queue_name = new_expr.arguments.first().and_then(|a| literal_str(a));

                // Look for `import * as processors from '...'` in namespace imports.
                // Detect it by finding a namespace import whose local name appears in the handler.
                // For simplicity, return any namespace import specifier that could be processors.
                // The handler body is not trivially inspectable here, so we return the first
                // namespace import that looks like processors (ending in /processors.mts or similar).
                let processors_specifier = namespace_imports
                    .values()
                    .find(|s| {
                        let name = s.rsplit('/').next().unwrap_or("");
                        name.starts_with("processors")
                    })
                    .cloned();

                let line = byte_offset_to_line(source, new_expr.span.start as usize);
                usage.worker_declarations.push(WorkerDeclaration {
                    queue_name,
                    processors_specifier,
                    line,
                });
            }
        }
        Expression::ChainExpression(chain) => {
            if let Some(e) = chain.expression.as_member_expression() {
                // Try to scan the object of the member expression
                scan_expr(e.object(), source, namespace_imports, usage);
            }
        }
        Expression::AwaitExpression(a) => {
            scan_expr(&a.argument, source, namespace_imports, usage);
        }
        Expression::ArrowFunctionExpression(arrow) => {
            let oxc::ast::ast::FunctionBody { statements, .. } = arrow.body.as_ref();
            for s in statements {
                scan_stmt(s, source, namespace_imports, usage);
            }
        }
        Expression::TSAsExpression(ts_as) => {
            scan_expr(&ts_as.expression, source, namespace_imports, usage);
        }
        Expression::TSNonNullExpression(ts_nn) => {
            scan_expr(&ts_nn.expression, source, namespace_imports, usage);
        }
        Expression::StaticMemberExpression(member) => {
            scan_expr(&member.object, source, namespace_imports, usage);
        }
        _ => {}
    }
}

/// If `expr` is `<identifier>.<method>(...)`, return `(identifier_name, method_name)`.
fn extract_member_call<'a>(expr: &'a Expression) -> Option<(String, &'a str)> {
    if let Expression::CallExpression(call) = expr {
        if let Expression::StaticMemberExpression(member) = &call.callee {
            if let Expression::Identifier(obj) = &member.object {
                return Some((obj.name.as_str().to_string(), member.property.name.as_str()));
            }
        }
    }
    None
}

fn literal_str(arg: &Argument) -> Option<String> {
    if let Argument::StringLiteral(s) = arg {
        return Some(s.value.as_str().to_string());
    }
    if let Some(e) = arg.as_expression() {
        return literal_str_expr(e);
    }
    None
}

fn literal_str_expr(expr: &Expression) -> Option<String> {
    if let Expression::StringLiteral(s) = expr {
        return Some(s.value.as_str().to_string());
    }
    None
}

#[cfg(test)]
mod tests;
