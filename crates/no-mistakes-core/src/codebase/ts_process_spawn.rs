use crate::codebase::ts_source::unwrap_ts_wrappers;
use oxc::allocator::Allocator;
use oxc::ast::ast::{
    Argument, ExportNamedDeclaration, Expression, ObjectPropertyKind, PropertyKey, Statement,
    TryStatement,
};
use oxc::parser::Parser;
use oxc::span::SourceType;
use std::path::{Path, PathBuf};

/// A resolved `(spawning_file, entry_file)` pair discovered via process-spawn analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct SpawnEdge {
    /// File that contains the spawn call or webServer declaration.
    pub spawner: PathBuf,
    /// Resolved entry file that is launched.
    pub entry: PathBuf,
}

/// Extract all spawn edges from `source` at `file_path`, resolving entry paths
/// relative to `root`.
///
/// Detects:
/// - Playwright `defineConfig({ webServer: [{ command: '<literal>', cwd?: '<literal>' }] })`
/// - `spawn('<cmd>', args?, opts?)`, `execFile('<cmd>', ...)`, `fork('<module>', ...)`
/// - `exec('<shell command>', ...)` with a string-literal shell command
///
/// String-literal commands are tokenized; env-var assignments (`VAR=value`) are
/// stripped, runtime prefixes (`node`, `tsx`, `npx`) are stripped, and the
/// remaining token is resolved as a file path.
///
/// Template literals whose expressions only appear before the file-path token
/// are also accepted — quasis are concatenated (interpolated values replaced with
/// empty string) and tokenized as above.
///
/// Non-literal arguments (dynamic expressions, ternaries, variable references)
/// are silently skipped — the `process-spawn-static` guardrail enforces literal
/// discipline in target codebases.
pub fn extract_spawn_edges(source: &str, file_path: &Path, root: &Path) -> Vec<SpawnEdge> {
    let allocator = Allocator::default();
    let source_type = SourceType::tsx();
    let ret = Parser::new(&allocator, source, source_type).parse();

    let mut results = Vec::new();

    for stmt in &ret.program.body {
        collect_from_stmt(stmt, source, file_path, root, &mut results);
    }

    results
}

fn collect_from_stmt(
    stmt: &Statement,
    source: &str,
    file_path: &Path,
    root: &Path,
    out: &mut Vec<SpawnEdge>,
) {
    match stmt {
        Statement::ExpressionStatement(s) => {
            collect_from_expr(&s.expression, source, file_path, root, out);
        }
        Statement::VariableDeclaration(v) => {
            for decl in &v.declarations {
                collect_from_optional_expr(decl.init.as_ref(), source, file_path, root, out);
            }
        }
        Statement::ReturnStatement(r) => {
            collect_from_optional_expr(r.argument.as_ref(), source, file_path, root, out);
        }
        Statement::BlockStatement(b) => {
            for s in &b.body {
                collect_from_stmt(s, source, file_path, root, out);
            }
        }
        Statement::FunctionDeclaration(f) => {
            if let Some(body) = &f.body {
                for s in &body.statements {
                    collect_from_stmt(s, source, file_path, root, out);
                }
            }
        }
        Statement::ExportNamedDeclaration(e) => {
            collect_from_export_named(e, source, file_path, root, out);
        }
        Statement::ExportDefaultDeclaration(e) => {
            collect_from_export_default(&e.declaration, source, file_path, root, out);
        }
        Statement::IfStatement(i) => {
            collect_from_stmt(&i.consequent, source, file_path, root, out);
            if let Some(alt) = &i.alternate {
                collect_from_stmt(alt, source, file_path, root, out);
            }
        }
        Statement::TryStatement(t) => {
            collect_from_try_stmt(t, source, file_path, root, out);
        }
        Statement::WhileStatement(w) => {
            collect_from_stmt(&w.body, source, file_path, root, out);
        }
        Statement::ForStatement(f) => {
            collect_from_stmt(&f.body, source, file_path, root, out);
        }
        Statement::ForInStatement(f) => {
            collect_from_stmt(&f.body, source, file_path, root, out);
        }
        Statement::ForOfStatement(f) => {
            collect_from_stmt(&f.body, source, file_path, root, out);
        }
        _ => {}
    }
}

fn collect_from_optional_expr(
    expr: Option<&Expression>,
    source: &str,
    file_path: &Path,
    root: &Path,
    out: &mut Vec<SpawnEdge>,
) {
    let _ = expr.map(|expr| collect_from_expr(expr, source, file_path, root, out));
}

fn collect_from_export_default(
    kind: &oxc::ast::ast::ExportDefaultDeclarationKind,
    source: &str,
    file_path: &Path,
    root: &Path,
    out: &mut Vec<SpawnEdge>,
) {
    match kind {
        oxc::ast::ast::ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
            if let Some(body) = &f.body {
                for s in &body.statements {
                    collect_from_stmt(s, source, file_path, root, out);
                }
            }
        }
        oxc::ast::ast::ExportDefaultDeclarationKind::ArrowFunctionExpression(a) => {
            for s in &a.body.statements {
                collect_from_stmt(s, source, file_path, root, out);
            }
        }
        oxc::ast::ast::ExportDefaultDeclarationKind::CallExpression(call)
            if callee_name(&call.callee) == Some("defineConfig") =>
        {
            extract_define_config_web_server(call, file_path, root, out);
        }
        _ => {}
    }
}

fn collect_from_expr(
    expr: &Expression,
    source: &str,
    file_path: &Path,
    root: &Path,
    out: &mut Vec<SpawnEdge>,
) {
    let expr = unwrap_ts_wrappers(expr);
    match expr {
        Expression::CallExpression(call) => {
            // Check for spawn/exec/execFile/fork at top level
            if let Some(fn_name) = callee_name(&call.callee) {
                match fn_name {
                    "spawn" | "execFile" | "fork" => {
                        // First arg is the command/module path
                        let entry = string_or_template_arg(&call.arguments, 0).and_then(|cmd| {
                            let cwd = extract_cwd_from_opts(&call.arguments, 2);
                            resolve_entry_file(&cmd, cwd.as_deref(), file_path, root)
                        });
                        if let Some(entry) = entry {
                            out.push(SpawnEdge {
                                spawner: file_path.to_path_buf(),
                                entry,
                            });
                        }
                    }
                    "exec" => {
                        // exec takes a shell command string; extract the file from it
                        if let Some(cmd) = string_or_template_arg(&call.arguments, 0) {
                            let cwd = extract_cwd_from_opts(&call.arguments, 1);
                            if let Some(entry) =
                                resolve_entry_file_from_shell(&cmd, cwd.as_deref(), file_path, root)
                            {
                                out.push(SpawnEdge {
                                    spawner: file_path.to_path_buf(),
                                    entry,
                                });
                            }
                        }
                    }
                    "defineConfig" => extract_define_config_web_server(call, file_path, root, out),
                    _ => {}
                }
            }
            // Recurse into arguments regardless
            for arg in &call.arguments {
                collect_from_optional_expr(arg.as_expression(), source, file_path, root, out);
            }
            collect_from_expr(&call.callee, source, file_path, root, out);
        }
        Expression::AwaitExpression(a) => {
            collect_from_expr(&a.argument, source, file_path, root, out)
        }
        Expression::ArrowFunctionExpression(a) => {
            for s in &a.body.statements {
                collect_from_stmt(s, source, file_path, root, out);
            }
        }
        Expression::ObjectExpression(obj) => {
            for prop in &obj.properties {
                if let ObjectPropertyKind::ObjectProperty(p) = prop {
                    // Look for a top-level webServer: [...] object property
                    if matches!(&p.key, PropertyKey::StaticIdentifier(id) if id.name.as_str() == "webServer")
                    {
                        extract_web_server(&p.value, file_path, root, out);
                    } else {
                        collect_from_expr(&p.value, source, file_path, root, out);
                    }
                }
            }
        }
        _ => {}
    }
}

/// Pull spawn edges out of a `webServer` value — either an array or a single object.
fn extract_web_server(expr: &Expression, file_path: &Path, root: &Path, out: &mut Vec<SpawnEdge>) {
    let expr = unwrap_ts_wrappers(expr);
    match expr {
        Expression::ArrayExpression(arr) => {
            for item in &arr.elements {
                extract_optional_web_server_entry(item.as_expression(), file_path, root, out);
            }
        }
        _ => extract_web_server_entry(expr, file_path, root, out),
    }
}

fn extract_optional_web_server_entry(
    expr: Option<&Expression>,
    file_path: &Path,
    root: &Path,
    out: &mut Vec<SpawnEdge>,
) {
    let _ = expr.map(|expr| extract_web_server_entry(expr, file_path, root, out));
}

fn extract_define_config_web_server(
    call: &oxc::ast::ast::CallExpression,
    file_path: &Path,
    root: &Path,
    out: &mut Vec<SpawnEdge>,
) {
    let Some(Argument::ObjectExpression(obj)) = call.arguments.first() else {
        return;
    };
    for p in obj.properties.iter().filter_map(|prop| match prop {
        ObjectPropertyKind::ObjectProperty(p) => Some(p),
        _ => None,
    }) {
        if matches!(&p.key, PropertyKey::StaticIdentifier(id) if id.name.as_str() == "webServer") {
            extract_web_server(&p.value, file_path, root, out);
        }
    }
}

fn extract_web_server_entry(
    expr: &Expression,
    file_path: &Path,
    root: &Path,
    out: &mut Vec<SpawnEdge>,
) {
    let expr = unwrap_ts_wrappers(expr);
    if let Expression::ObjectExpression(obj) = expr {
        let mut command: Option<String> = None;
        let mut cwd: Option<PathBuf> = None;

        for prop in &obj.properties {
            if let ObjectPropertyKind::ObjectProperty(p) = prop {
                let key = match &p.key {
                    PropertyKey::StaticIdentifier(id) => id.name.as_str(),
                    _ => continue,
                };
                match key {
                    "command" => {
                        command = string_or_template_literal(&p.value);
                    }
                    "cwd" => {
                        // cwd may be a dynamic join() — only take literal strings
                        assign_literal_cwd(&mut cwd, &p.value);
                    }
                    _ => {}
                }
            }
        }

        if let Some(cmd) = command {
            let cwd_str = cwd.as_deref().and_then(|p| p.to_str());
            if let Some(entry) = resolve_entry_file_from_shell(&cmd, cwd_str, file_path, root) {
                out.push(SpawnEdge {
                    spawner: file_path.to_path_buf(),
                    entry,
                });
            }
        }
    }
}

fn collect_from_export_named(
    e: &ExportNamedDeclaration,
    source: &str,
    file_path: &Path,
    root: &Path,
    out: &mut Vec<SpawnEdge>,
) {
    let Some(decl) = &e.declaration else { return };
    match decl {
        oxc::ast::ast::Declaration::VariableDeclaration(v) => {
            for d in &v.declarations {
                collect_from_optional_expr(d.init.as_ref(), source, file_path, root, out);
            }
        }
        oxc::ast::ast::Declaration::FunctionDeclaration(f) => {
            if let Some(body) = &f.body {
                for s in &body.statements {
                    collect_from_stmt(s, source, file_path, root, out);
                }
            }
        }
        _ => {}
    }
}

fn collect_from_try_stmt(
    t: &TryStatement,
    source: &str,
    file_path: &Path,
    root: &Path,
    out: &mut Vec<SpawnEdge>,
) {
    for s in &t.block.body {
        collect_from_stmt(s, source, file_path, root, out);
    }
    if let Some(handler) = &t.handler {
        for s in &handler.body.body {
            collect_from_stmt(s, source, file_path, root, out);
        }
    }
    if let Some(finalizer) = &t.finalizer {
        for s in &finalizer.body {
            collect_from_stmt(s, source, file_path, root, out);
        }
    }
}

fn assign_literal_cwd(cwd: &mut Option<PathBuf>, expr: &Expression) {
    *cwd = literal_string(expr).map(PathBuf::from);
}

// ── String extraction helpers ─────────────────────────────────────────────────

/// Extract a string literal or template literal (quasis concatenated) from an argument.
fn string_or_template_arg(args: &[Argument], index: usize) -> Option<String> {
    let arg = args.get(index)?;
    let expr = arg.as_expression()?;
    string_or_template_literal(expr)
}

/// Extract a string literal or template literal (quasis concatenated) from an expression.
fn string_or_template_literal(expr: &Expression) -> Option<String> {
    let expr = unwrap_ts_wrappers(expr);
    match expr {
        Expression::StringLiteral(s) => Some(s.value.as_str().to_string()),
        Expression::TemplateLiteral(tl) => {
            // Concatenate quasi strings (static parts), replacing interpolations with "".
            let parts: Vec<&str> = tl
                .quasis
                .iter()
                .filter_map(|q| q.value.cooked.as_deref())
                .collect();
            Some(parts.join(""))
        }
        _ => None,
    }
}

fn literal_string(expr: &Expression) -> Option<String> {
    let expr = unwrap_ts_wrappers(expr);
    if let Expression::StringLiteral(s) = expr {
        Some(s.value.as_str().to_string())
    } else {
        None
    }
}

/// Extract `cwd` from the opts object at `args[opts_index]`.
fn extract_cwd_from_opts(args: &[Argument], opts_index: usize) -> Option<String> {
    let obj = match args
        .get(opts_index)?
        .as_expression()
        .map(unwrap_ts_wrappers)
    {
        Some(Expression::ObjectExpression(obj)) => obj,
        _ => return None,
    };
    obj.properties
        .iter()
        .filter_map(|prop| match prop {
            ObjectPropertyKind::ObjectProperty(p) => Some(p),
            _ => None,
        })
        .find_map(|p| {
            matches!(&p.key, PropertyKey::StaticIdentifier(id) if id.name.as_str() == "cwd")
                .then(|| literal_string(&p.value))
                .flatten()
        })
}

/// Attempt to get the short name of a callee (last identifier in a chain).
fn callee_name<'a>(expr: &'a Expression<'a>) -> Option<&'a str> {
    let expr = unwrap_ts_wrappers(expr);
    match expr {
        Expression::Identifier(id) => Some(id.name.as_str()),
        Expression::StaticMemberExpression(m) => Some(m.property.name.as_str()),
        _ => None,
    }
}

// ── Entry file resolution ─────────────────────────────────────────────────────

/// Tokenize a shell command string and find the entry file.
///
/// Algorithm:
/// 1. Split by whitespace. (Quoted paths with spaces are not supported — the
///    `process-spawn-static` guardrail enforces simple, space-free commands.)
/// 2. Skip `KEY=value` tokens (env var assignments).
/// 3. Skip known runtime prefixes: `node`, `tsx`, `npx`, `pnpm`, `npm`, `yarn`.
/// 4. Return the first remaining token that looks like a file path (contains a
///    `.` extension or `/`). Extension-less entry points (e.g. `node server`) are
///    not resolved; the target codebase uses explicit `.mts` extensions everywhere.
fn resolve_entry_file_from_shell(
    cmd: &str,
    cwd: Option<&str>,
    file_path: &Path,
    root: &Path,
) -> Option<PathBuf> {
    let tokens: Vec<&str> = cmd.split_whitespace().collect();
    let file_token = tokens
        .iter()
        .skip_while(|t| {
            // Skip env var assignments like VAR=value or VAR=
            if t.contains('=') {
                return true;
            }
            // Skip runtime/tool prefixes
            matches!(
                **t,
                "node" | "tsx" | "npx" | "pnpm" | "npm" | "yarn" | "bunx" | "bun" | "run"
            )
        })
        .find(|t| looks_like_file_path(t))?;

    resolve_entry_file(file_token, cwd, file_path, root)
}

/// Resolve a file path token against `cwd ?? config_dir ?? root`.
fn resolve_entry_file(
    token: &str,
    cwd: Option<&str>,
    file_path: &Path,
    root: &Path,
) -> Option<PathBuf> {
    let base = if let Some(cwd) = cwd {
        let cwd_path = PathBuf::from(cwd);
        if cwd_path.is_absolute() {
            cwd_path
        } else {
            root.join(cwd_path)
        }
    } else {
        let fallback = root.to_path_buf();
        file_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or(fallback)
    };

    let candidate = base.join(token);
    if candidate.is_file() {
        return Some(candidate);
    }

    // Also try relative to root
    let root_candidate = root.join(token);
    if root_candidate.is_file() {
        return Some(root_candidate);
    }

    None
}

fn looks_like_file_path(token: &str) -> bool {
    let has_file_shape = token.contains('.') || token.contains('/');
    let is_flag = token.starts_with('-');
    let is_url = token.starts_with("http");
    has_file_shape && !is_flag && !is_url
}

#[cfg(test)]
mod tests;
