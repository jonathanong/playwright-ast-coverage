use crate::codebase::ts_source::unwrap_ts_wrappers;
use oxc::allocator::Allocator;
use oxc::ast::ast::{Argument, Expression, ObjectPropertyKind, PropertyKey, Statement};
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
                if let Some(init) = &decl.init {
                    collect_from_expr(init, source, file_path, root, out);
                }
            }
        }
        Statement::ReturnStatement(r) => {
            if let Some(e) = &r.argument {
                collect_from_expr(e, source, file_path, root, out);
            }
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
            if let Some(decl) = &e.declaration {
                match decl {
                    oxc::ast::ast::Declaration::VariableDeclaration(v) => {
                        for d in &v.declarations {
                            if let Some(init) = &d.init {
                                collect_from_expr(init, source, file_path, root, out);
                            }
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
        oxc::ast::ast::ExportDefaultDeclarationKind::CallExpression(call) => {
            if let Some(fn_name) = callee_name(&call.callee) {
                if fn_name == "defineConfig" {
                    if let Some(Argument::ObjectExpression(obj)) = call.arguments.first() {
                        for prop in &obj.properties {
                            if let ObjectPropertyKind::ObjectProperty(p) = prop {
                                if matches!(&p.key, PropertyKey::StaticIdentifier(id) if id.name.as_str() == "webServer")
                                {
                                    extract_web_server(&p.value, file_path, root, out);
                                }
                            }
                        }
                    }
                }
            }
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
                        if let Some(cmd) = string_or_template_arg(&call.arguments, 0) {
                            let cwd = extract_cwd_from_opts(&call.arguments, 2);
                            if let Some(entry) =
                                resolve_entry_file(&cmd, cwd.as_deref(), file_path, root)
                            {
                                out.push(SpawnEdge {
                                    spawner: file_path.to_path_buf(),
                                    entry,
                                });
                            }
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
                    "defineConfig" => {
                        // Playwright defineConfig({ webServer: [...] })
                        if let Some(Argument::ObjectExpression(obj)) = call.arguments.first() {
                            for prop in &obj.properties {
                                if let ObjectPropertyKind::ObjectProperty(p) = prop {
                                    if matches!(&p.key, PropertyKey::StaticIdentifier(id) if id.name.as_str() == "webServer")
                                    {
                                        extract_web_server(&p.value, file_path, root, out);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            // Recurse into arguments regardless
            for arg in &call.arguments {
                if let Some(e) = arg.as_expression() {
                    collect_from_expr(e, source, file_path, root, out);
                }
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
                if let Some(e) = item.as_expression() {
                    extract_web_server_entry(e, file_path, root, out);
                }
            }
        }
        _ => extract_web_server_entry(expr, file_path, root, out),
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
                        if let Some(s) = literal_string(&p.value) {
                            cwd = Some(PathBuf::from(s));
                        }
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

// ── String extraction helpers ─────────────────────────────────────────────────

/// Extract a string literal or template literal (quasis concatenated) from an argument.
fn string_or_template_arg(args: &[Argument], index: usize) -> Option<String> {
    let expr = args.get(index)?.as_expression()?;
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
    let opts = args.get(opts_index)?.as_expression()?;
    let opts = unwrap_ts_wrappers(opts);
    if let Expression::ObjectExpression(obj) = opts {
        for prop in &obj.properties {
            if let ObjectPropertyKind::ObjectProperty(p) = prop {
                if matches!(&p.key, PropertyKey::StaticIdentifier(id) if id.name.as_str() == "cwd")
                {
                    return literal_string(&p.value);
                }
            }
        }
    }
    None
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
        file_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| root.to_path_buf())
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
    // Must contain a dot (extension) or slash (path separator)
    (token.contains('.') || token.contains('/'))
        // Must not start with '-' (flag)
        && !token.starts_with('-')
        // Must not be a URL
        && !token.starts_with("http")
}

#[cfg(test)]
mod tests;
