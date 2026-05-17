use crate::codebase::ts_resolver;
use crate::codebase::ts_source::byte_offset_to_line;
use oxc::allocator::Allocator;
use oxc::ast::ast::{Expression, ImportDeclarationSpecifier, ModuleExportName, Statement};
use oxc::parser::Parser;
use oxc::span::SourceType;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

/// BFS from `entrypoint`, following imports through `tsconfig` aliases.
/// Returns a set of canonicalized absolute paths for all reachable files.
pub fn bfs_reachable(entrypoint: &Path, tsconfig: &ts_resolver::TsConfig) -> HashSet<PathBuf> {
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut queue: VecDeque<PathBuf> = VecDeque::new();

    let start = entrypoint
        .canonicalize()
        .unwrap_or(entrypoint.to_path_buf());
    queue.push_back(start);

    while let Some(file) = queue.pop_front() {
        if visited.contains(&file) {
            continue;
        }
        visited.insert(file.clone());

        let source = match std::fs::read_to_string(&file) {
            Ok(s) => s,
            Err(_) => continue,
        };

        for specifier in collect_import_specifiers(&source) {
            if let Some(resolved) = ts_resolver::resolve_import(&specifier, &file, tsconfig) {
                let canonical = resolved.canonicalize().unwrap_or(resolved);
                if !visited.contains(&canonical) {
                    queue.push_back(canonical);
                }
            }
        }
    }

    visited
}

/// Parse `source` and collect all import/export module specifier strings.
pub fn collect_import_specifiers(source: &str) -> Vec<String> {
    let allocator = Allocator::default();
    let source_type = SourceType::ts();
    let ret = Parser::new(&allocator, source, source_type).parse();
    let mut specifiers = Vec::new();

    for stmt in &ret.program.body {
        match stmt {
            Statement::ImportDeclaration(import_decl) => {
                specifiers.push(import_decl.source.value.to_string());
            }
            Statement::ExportNamedDeclaration(export) => {
                if let Some(src) = &export.source {
                    specifiers.push(src.value.to_string());
                }
            }
            Statement::ExportAllDeclaration(export) => {
                specifiers.push(export.source.value.to_string());
            }
            _ => {}
        }
    }

    specifiers
}

/// Parse `source` and determine if it contains a call to `factory_function`
/// imported from `factory_specifier`. Returns the 1-based line number if found.
pub fn find_create_queue_line(
    source: &str,
    factory_specifier: &str,
    factory_function: &str,
) -> Option<u32> {
    let allocator = Allocator::default();
    let source_type = SourceType::ts();
    let ret = Parser::new(&allocator, source, source_type).parse();

    let mut bindings: HashMap<String, (String, String)> = HashMap::new();
    for stmt in &ret.program.body {
        if let Statement::ImportDeclaration(import_decl) = stmt {
            let src = import_decl.source.value.as_str();
            if let Some(specifiers) = &import_decl.specifiers {
                for specifier in specifiers {
                    if let ImportDeclarationSpecifier::ImportSpecifier(spec) = specifier {
                        let imported_name = module_export_name_str(&spec.imported);
                        let local_name = spec.local.name.as_str().to_string();
                        bindings.insert(local_name, (src.to_string(), imported_name));
                    }
                }
            }
        }
    }

    for stmt in &ret.program.body {
        if let Some(line) = check_stmt_for_create_queue(
            stmt,
            source,
            &bindings,
            factory_specifier,
            factory_function,
        ) {
            return Some(line);
        }
    }

    None
}

fn module_export_name_str(name: &ModuleExportName) -> String {
    name.name().as_str().to_string()
}

fn check_stmt_for_create_queue(
    stmt: &Statement,
    source: &str,
    bindings: &HashMap<String, (String, String)>,
    factory_specifier: &str,
    factory_function: &str,
) -> Option<u32> {
    match stmt {
        Statement::ExpressionStatement(expr_stmt) => check_expr_for_create_queue(
            &expr_stmt.expression,
            source,
            bindings,
            factory_specifier,
            factory_function,
        ),
        Statement::VariableDeclaration(var_decl) => {
            for decl in &var_decl.declarations {
                if let Some(init) = &decl.init {
                    if let Some(line) = check_expr_for_create_queue(
                        init,
                        source,
                        bindings,
                        factory_specifier,
                        factory_function,
                    ) {
                        return Some(line);
                    }
                }
            }
            None
        }
        Statement::ExportNamedDeclaration(export) => {
            if let Some(decl) = &export.declaration {
                match decl {
                    oxc::ast::ast::Declaration::VariableDeclaration(var_decl) => {
                        for d in &var_decl.declarations {
                            if let Some(init) = &d.init {
                                if let Some(line) = check_expr_for_create_queue(
                                    init,
                                    source,
                                    bindings,
                                    factory_specifier,
                                    factory_function,
                                ) {
                                    return Some(line);
                                }
                            }
                        }
                        None
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

fn check_expr_for_create_queue(
    expr: &Expression,
    source: &str,
    bindings: &HashMap<String, (String, String)>,
    factory_specifier: &str,
    factory_function: &str,
) -> Option<u32> {
    match expr {
        Expression::CallExpression(call_expr) => {
            let callee_name = match &call_expr.callee {
                Expression::Identifier(id) => Some(id.name.as_str()),
                _ => None,
            };

            if let Some(name) = callee_name {
                if let Some((src, imported)) = bindings.get(name) {
                    if src == factory_specifier && imported == factory_function {
                        let line = byte_offset_to_line(source, call_expr.span.start as usize);
                        return Some(line);
                    }
                }
            }

            if let Some(line) = call_expr.arguments.iter().find_map(|arg| {
                let oxc::ast::ast::Argument::CallExpression(inner) = arg else {
                    return None;
                };
                let Expression::Identifier(id) = &inner.callee else {
                    return None;
                };
                bindings.get(id.name.as_str()).and_then(|(src, imported)| {
                    (src == factory_specifier && imported == factory_function)
                        .then(|| byte_offset_to_line(source, inner.span.start as usize))
                })
            }) {
                return Some(line);
            }
            None
        }
        Expression::TSAsExpression(ts_as) => check_expr_for_create_queue(
            &ts_as.expression,
            source,
            bindings,
            factory_specifier,
            factory_function,
        ),
        Expression::TSNonNullExpression(ts_nn) => check_expr_for_create_queue(
            &ts_nn.expression,
            source,
            bindings,
            factory_specifier,
            factory_function,
        ),
        _ => None,
    }
}

fn collect_const_string_bindings(stmts: &[Statement]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for stmt in stmts {
        let var_decl = match stmt {
            Statement::VariableDeclaration(v) => v,
            Statement::ExportNamedDeclaration(e) => {
                if let Some(oxc::ast::ast::Declaration::VariableDeclaration(v)) = &e.declaration {
                    v
                } else {
                    continue;
                }
            }
            _ => continue,
        };
        if var_decl.kind != oxc::ast::ast::VariableDeclarationKind::Const {
            continue;
        }
        for decl in &var_decl.declarations {
            let name = match &decl.id {
                oxc::ast::ast::BindingPattern::BindingIdentifier(id) => {
                    id.name.as_str().to_string()
                }
                _ => continue,
            };
            if let Some(Expression::StringLiteral(s)) = &decl.init {
                map.insert(name, s.value.as_str().to_string());
            }
        }
    }
    map
}

/// Parse `source` and return the queue name from a `createQueue(name, ...)` call.
/// Resolves top-level `const NAME = "..."` bindings when the first argument is an identifier.
/// Returns `Some("<unknown>")` when the call is found but the name cannot be statically resolved.
pub fn find_queue_name(
    source: &str,
    factory_specifier: &str,
    factory_function: &str,
) -> Option<String> {
    let allocator = Allocator::default();
    let source_type = SourceType::ts();
    let ret = Parser::new(&allocator, source, source_type).parse();

    let mut bindings: HashMap<String, (String, String)> = HashMap::new();
    for stmt in &ret.program.body {
        if let Statement::ImportDeclaration(import_decl) = stmt {
            let src = import_decl.source.value.as_str();
            if let Some(specifiers) = &import_decl.specifiers {
                for specifier in specifiers {
                    if let ImportDeclarationSpecifier::ImportSpecifier(spec) = specifier {
                        let imported_name = module_export_name_str(&spec.imported);
                        let local_name = spec.local.name.as_str().to_string();
                        bindings.insert(local_name, (src.to_string(), imported_name));
                    }
                }
            }
        }
    }

    let const_strings = collect_const_string_bindings(&ret.program.body);

    for stmt in &ret.program.body {
        if let Some(name) = find_queue_name_in_stmt(
            stmt,
            &bindings,
            &const_strings,
            factory_specifier,
            factory_function,
        ) {
            return Some(name);
        }
    }
    None
}

fn find_queue_name_in_stmt(
    stmt: &Statement,
    bindings: &HashMap<String, (String, String)>,
    const_strings: &HashMap<String, String>,
    factory_specifier: &str,
    factory_function: &str,
) -> Option<String> {
    match stmt {
        Statement::ExpressionStatement(e) => find_queue_name_in_expr(
            &e.expression,
            bindings,
            const_strings,
            factory_specifier,
            factory_function,
        ),
        Statement::VariableDeclaration(v) => {
            for decl in &v.declarations {
                if let Some(init) = &decl.init {
                    if let Some(name) = find_queue_name_in_expr(
                        init,
                        bindings,
                        const_strings,
                        factory_specifier,
                        factory_function,
                    ) {
                        return Some(name);
                    }
                }
            }
            None
        }
        Statement::ExportNamedDeclaration(e) => {
            if let Some(oxc::ast::ast::Declaration::VariableDeclaration(v)) = &e.declaration {
                for d in &v.declarations {
                    if let Some(init) = &d.init {
                        if let Some(name) = find_queue_name_in_expr(
                            init,
                            bindings,
                            const_strings,
                            factory_specifier,
                            factory_function,
                        ) {
                            return Some(name);
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn find_queue_name_in_expr(
    expr: &Expression,
    bindings: &HashMap<String, (String, String)>,
    const_strings: &HashMap<String, String>,
    factory_specifier: &str,
    factory_function: &str,
) -> Option<String> {
    match expr {
        Expression::CallExpression(call_expr) => {
            let callee_name = match &call_expr.callee {
                Expression::Identifier(id) => Some(id.name.as_str()),
                _ => None,
            }?;
            if let Some((src, imported)) = bindings.get(callee_name) {
                if src == factory_specifier && imported == factory_function {
                    let resolved = match call_expr.arguments.first() {
                        Some(oxc::ast::ast::Argument::StringLiteral(s)) => {
                            s.value.as_str().to_string()
                        }
                        Some(oxc::ast::ast::Argument::Identifier(id)) => const_strings
                            .get(id.name.as_str())
                            .cloned()
                            .unwrap_or_else(|| "<unknown>".to_string()),
                        _ => "<unknown>".to_string(),
                    };
                    return Some(resolved);
                }
            }
            None
        }
        Expression::TSAsExpression(ts_as) => find_queue_name_in_expr(
            &ts_as.expression,
            bindings,
            const_strings,
            factory_specifier,
            factory_function,
        ),
        Expression::TSNonNullExpression(ts_nn) => find_queue_name_in_expr(
            &ts_nn.expression,
            bindings,
            const_strings,
            factory_specifier,
            factory_function,
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
