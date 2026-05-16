use crate::codebase::ts_source::{byte_offset_to_line, is_skipped_dir};
use oxc::allocator::Allocator;
use oxc::ast::ast::{
    Argument, BindingPattern, Expression, ForStatementInit, ForStatementLeft, Statement,
    TemplateLiteral, VariableDeclarationKind,
};
use oxc::parser::Parser;
use oxc::span::SourceType;
use std::path::PathBuf;

const HTTP_VERBS: &[&str] = &["get", "post", "put", "patch", "delete", "head", "options"];

/// Scan `source` for `<object>.route('<literal>').<method>(...)` chains.
/// Returns `(pattern, line_number)` pairs.
pub fn extract_backend_routes(source: &str, register_object: &str) -> Vec<(String, u32)> {
    let allocator = Allocator::default();
    let source_type = SourceType::ts();
    let ret = Parser::new(&allocator, source, source_type).parse();
    let mut results = Vec::new();

    for stmt in &ret.program.body {
        collect_from_statement(stmt, source, register_object, true, &mut results);
    }

    results
}

/// Scan all `.mts`/`.ts` files under `dir` for backend route definitions using
/// `register_object` as the chain object. Returns `(file, pattern)` pairs.
pub fn collect_backend_routes_in_dir(
    dir: &std::path::Path,
    register_object: &str,
    pattern_globset: &globset::GlobSet,
) -> Vec<(PathBuf, String)> {
    use walkdir::WalkDir;
    let mut results = Vec::new();

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| !is_skipped_dir(e.file_name().to_str().unwrap_or("")))
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let rel = match path.strip_prefix(dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if !pattern_globset.is_match(rel) {
            continue;
        }
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        for (route, _line) in extract_backend_routes(&source, register_object) {
            results.push((path.to_path_buf(), route));
        }
    }

    results
}

/// Collect backend route definitions from an already-discovered file list.
pub fn collect_backend_routes_from_files(
    root: &std::path::Path,
    files: &[PathBuf],
    register_object: &str,
    pattern_globset: &globset::GlobSet,
) -> Vec<(PathBuf, String)> {
    let mut results = Vec::new();

    for path in files {
        if !path.is_file() {
            continue;
        }
        let rel = match path.strip_prefix(root) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if !pattern_globset.is_match(rel) {
            continue;
        }
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        for (route, _line) in extract_backend_routes(&source, register_object) {
            results.push((path.clone(), route));
        }
    }

    results
}

fn collect_from_statement(
    stmt: &Statement,
    source: &str,
    register_object: &str,
    register_object_in_scope: bool,
    results: &mut Vec<(String, u32)>,
) {
    match stmt {
        Statement::ExpressionStatement(expr_stmt) => {
            collect_from_expression(
                &expr_stmt.expression,
                source,
                register_object,
                register_object_in_scope,
                results,
            );
        }
        Statement::VariableDeclaration(var_decl) => {
            let register_object_in_scope = register_object_in_scope
                && !var_decl
                    .declarations
                    .iter()
                    .any(|decl| binding_pattern_contains_name(&decl.id, register_object));
            for decl in &var_decl.declarations {
                if let Some(init) = &decl.init {
                    collect_from_expression(
                        init,
                        source,
                        register_object,
                        register_object_in_scope,
                        results,
                    );
                }
            }
        }
        Statement::BlockStatement(block) => {
            let register_object_in_scope =
                register_object_in_scope && !statements_shadow_name(&block.body, register_object);
            for s in &block.body {
                collect_from_statement(
                    s,
                    source,
                    register_object,
                    register_object_in_scope,
                    results,
                );
            }
        }
        Statement::FunctionDeclaration(func) => {
            if let Some(body) = &func.body {
                let register_object_in_scope = register_object_in_scope
                    && !function_name_shadows_name(func, register_object)
                    && !params_shadow_name(&func.params, register_object)
                    && !statements_shadow_name(&body.statements, register_object);
                for s in &body.statements {
                    collect_from_statement(
                        s,
                        source,
                        register_object,
                        register_object_in_scope,
                        results,
                    );
                }
            }
        }
        Statement::ExportNamedDeclaration(export) => {
            if let Some(decl) = &export.declaration {
                match decl {
                    oxc::ast::ast::Declaration::VariableDeclaration(var_decl) => {
                        let register_object_in_scope = register_object_in_scope
                            && !var_decl.declarations.iter().any(|decl| {
                                binding_pattern_contains_name(&decl.id, register_object)
                            });
                        for d in &var_decl.declarations {
                            if let Some(init) = &d.init {
                                collect_from_expression(
                                    init,
                                    source,
                                    register_object,
                                    register_object_in_scope,
                                    results,
                                );
                            }
                        }
                    }
                    oxc::ast::ast::Declaration::FunctionDeclaration(func) => {
                        if let Some(body) = &func.body {
                            let register_object_in_scope = register_object_in_scope
                                && !function_name_shadows_name(func, register_object)
                                && !params_shadow_name(&func.params, register_object)
                                && !statements_shadow_name(&body.statements, register_object);
                            for s in &body.statements {
                                collect_from_statement(
                                    s,
                                    source,
                                    register_object,
                                    register_object_in_scope,
                                    results,
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn collect_from_expression(
    expr: &Expression,
    source: &str,
    register_object: &str,
    register_object_in_scope: bool,
    results: &mut Vec<(String, u32)>,
) {
    if let Expression::CallExpression(call) = expr {
        if let Some(member) = call.callee.as_member_expression() {
            if let Some(verb) = member.static_property_name() {
                if HTTP_VERBS.contains(&verb) {
                    let line = byte_offset_to_line(source, call.span.start as usize);
                    if register_object_in_scope {
                        if let Some(route_pattern) =
                            direct_route_arg(call, member.object(), register_object).or_else(|| {
                                extract_route_from_chain(member.object(), register_object)
                            })
                        {
                            results.push((route_pattern, line));
                        }
                    }
                    collect_from_expression(
                        member.object(),
                        source,
                        register_object,
                        register_object_in_scope,
                        results,
                    );
                    return;
                }
            }
        }
        collect_from_expression(
            &call.callee,
            source,
            register_object,
            register_object_in_scope,
            results,
        );
        for arg in &call.arguments {
            if let Some(expr) = arg.as_expression() {
                collect_from_expression(
                    expr,
                    source,
                    register_object,
                    register_object_in_scope,
                    results,
                );
            }
        }
    }
}

fn statements_shadow_name(stmts: &[Statement], name: &str) -> bool {
    stmts.iter().any(|stmt| match stmt {
        Statement::VariableDeclaration(var_decl) => {
            variable_declaration_shadows_name(var_decl, name)
        }
        Statement::FunctionDeclaration(func) => func
            .id
            .as_ref()
            .map(|id| id.name.as_str() == name)
            .unwrap_or(false),
        Statement::ClassDeclaration(class) => class
            .id
            .as_ref()
            .map(|id| id.name.as_str() == name)
            .unwrap_or(false),
        _ => statement_has_function_scope_var_shadow(stmt, name),
    })
}

fn statement_has_function_scope_var_shadow(stmt: &Statement, name: &str) -> bool {
    match stmt {
        Statement::VariableDeclaration(var_decl) => {
            var_decl.kind == VariableDeclarationKind::Var
                && variable_declaration_shadows_name(var_decl, name)
        }
        Statement::BlockStatement(block) => block
            .body
            .iter()
            .any(|stmt| statement_has_function_scope_var_shadow(stmt, name)),
        Statement::IfStatement(if_stmt) => {
            statement_has_function_scope_var_shadow(&if_stmt.consequent, name)
                || if_stmt
                    .alternate
                    .as_ref()
                    .map(|alt| statement_has_function_scope_var_shadow(alt, name))
                    .unwrap_or(false)
        }
        Statement::ForStatement(for_stmt) => {
            let init_shadows = matches!(
                &for_stmt.init,
                Some(ForStatementInit::VariableDeclaration(var_decl))
                    if var_decl.kind == VariableDeclarationKind::Var
                        && variable_declaration_shadows_name(var_decl, name)
            );
            init_shadows || statement_has_function_scope_var_shadow(&for_stmt.body, name)
        }
        Statement::ForInStatement(for_stmt) => {
            for_left_var_declaration_shadows_name(&for_stmt.left, name)
                || statement_has_function_scope_var_shadow(&for_stmt.body, name)
        }
        Statement::ForOfStatement(for_stmt) => {
            for_left_var_declaration_shadows_name(&for_stmt.left, name)
                || statement_has_function_scope_var_shadow(&for_stmt.body, name)
        }
        Statement::WhileStatement(while_stmt) => {
            statement_has_function_scope_var_shadow(&while_stmt.body, name)
        }
        Statement::DoWhileStatement(do_while_stmt) => {
            statement_has_function_scope_var_shadow(&do_while_stmt.body, name)
        }
        Statement::SwitchStatement(switch_stmt) => switch_stmt.cases.iter().any(|case| {
            case.consequent
                .iter()
                .any(|stmt| statement_has_function_scope_var_shadow(stmt, name))
        }),
        Statement::TryStatement(try_stmt) => {
            try_stmt
                .block
                .body
                .iter()
                .any(|stmt| statement_has_function_scope_var_shadow(stmt, name))
                || try_stmt
                    .handler
                    .as_ref()
                    .map(|handler| {
                        handler
                            .body
                            .body
                            .iter()
                            .any(|stmt| statement_has_function_scope_var_shadow(stmt, name))
                    })
                    .unwrap_or(false)
                || try_stmt
                    .finalizer
                    .as_ref()
                    .map(|finalizer| {
                        finalizer
                            .body
                            .iter()
                            .any(|stmt| statement_has_function_scope_var_shadow(stmt, name))
                    })
                    .unwrap_or(false)
        }
        _ => false,
    }
}

fn for_left_var_declaration_shadows_name(left: &ForStatementLeft, name: &str) -> bool {
    matches!(
        left,
        ForStatementLeft::VariableDeclaration(var_decl)
            if var_decl.kind == VariableDeclarationKind::Var
                && variable_declaration_shadows_name(var_decl, name)
    )
}

fn function_name_shadows_name(func: &oxc::ast::ast::Function, name: &str) -> bool {
    func.id
        .as_ref()
        .map(|id| id.name.as_str() == name)
        .unwrap_or(false)
}

fn variable_declaration_shadows_name(
    var_decl: &oxc::ast::ast::VariableDeclaration,
    name: &str,
) -> bool {
    var_decl
        .declarations
        .iter()
        .any(|decl| binding_pattern_contains_name(&decl.id, name))
}

fn params_shadow_name(params: &oxc::ast::ast::FormalParameters, name: &str) -> bool {
    params
        .items
        .iter()
        .any(|param| binding_pattern_contains_name(&param.pattern, name))
        || params
            .rest
            .as_ref()
            .map(|rest| binding_pattern_contains_name(&rest.rest.argument, name))
            .unwrap_or(false)
}

fn binding_pattern_contains_name(pattern: &BindingPattern, name: &str) -> bool {
    match pattern {
        BindingPattern::BindingIdentifier(id) => id.name.as_str() == name,
        BindingPattern::ObjectPattern(obj) => {
            obj.properties
                .iter()
                .any(|prop| binding_pattern_contains_name(&prop.value, name))
                || obj
                    .rest
                    .as_ref()
                    .map(|rest| binding_pattern_contains_name(&rest.argument, name))
                    .unwrap_or(false)
        }
        BindingPattern::ArrayPattern(arr) => {
            arr.elements
                .iter()
                .flatten()
                .any(|element| binding_pattern_contains_name(element, name))
                || arr
                    .rest
                    .as_ref()
                    .map(|rest| binding_pattern_contains_name(&rest.argument, name))
                    .unwrap_or(false)
        }
        BindingPattern::AssignmentPattern(assign) => {
            binding_pattern_contains_name(&assign.left, name)
        }
    }
}

fn extract_route_from_chain(expr: &Expression, register_object: &str) -> Option<String> {
    if let Expression::CallExpression(call) = expr {
        if let Some(member) = call.callee.as_member_expression() {
            let prop = member.static_property_name().unwrap_or("");
            if prop == "route" {
                if let Expression::Identifier(ident) = member.object() {
                    if ident.name.as_str() == register_object {
                        return route_arg(call.arguments.first());
                    }
                }
            } else {
                return extract_route_from_chain(member.object(), register_object);
            }
        }
    }
    None
}

fn direct_route_arg(
    call: &oxc::ast::ast::CallExpression,
    callee_object: &Expression,
    register_object: &str,
) -> Option<String> {
    let Expression::Identifier(ident) = callee_object else {
        return None;
    };
    if ident.name.as_str() != register_object {
        return None;
    }
    route_arg(call.arguments.first())
}

fn route_arg(arg: Option<&Argument>) -> Option<String> {
    match arg? {
        Argument::StringLiteral(s) => Some(s.value.as_str().to_string()),
        Argument::TemplateLiteral(tpl) if tpl.expressions.is_empty() => {
            Some(static_template_literal(tpl))
        }
        _ => None,
    }
}

fn static_template_literal(tpl: &TemplateLiteral) -> String {
    tpl.quasis
        .iter()
        .filter_map(|quasi| quasi.value.cooked.as_deref())
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests;
