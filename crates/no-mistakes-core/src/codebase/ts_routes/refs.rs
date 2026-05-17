use crate::codebase::ts_source::byte_offset_to_line;
use oxc::allocator::Allocator;
use oxc::ast::ast::{
    Argument, BindingPattern, Expression, ForStatementInit, ForStatementLeft,
    ImportDeclarationSpecifier, JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXChild,
    JSXElement, JSXExpression, ObjectPropertyKind, PropertyKey, Statement, TemplateLiteral,
    VariableDeclarationKind,
};
use oxc::parser::Parser;
use oxc::span::SourceType;
use std::collections::HashSet;
use std::path::Path;

/// A route reference found in source code.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteRef {
    pub pattern: String,
    pub file: String,
    pub line: u32,
}

/// Scan `source` for route references. Returns a Vec of RouteRef.
pub fn extract_route_refs(source: &str, file: &str) -> Vec<RouteRef> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(Path::new(file)).unwrap_or(SourceType::tsx());
    let ret = Parser::new(&allocator, source, source_type).parse();

    let mut router_bindings = collect_import_bindings(&ret.program.body);
    collect_router_bindings_for_scope(&ret.program.body, &mut router_bindings);

    let mut refs = Vec::new();
    for stmt in &ret.program.body {
        collect_from_statement(stmt, source, file, &mut router_bindings, &mut refs);
    }

    refs
}

#[derive(Clone, Default)]
struct RouterBindings<'a> {
    objects: HashSet<&'a str>,
    methods: HashSet<&'a str>,
    redirects: HashSet<&'a str>,
}

fn collect_import_bindings<'a>(stmts: &'a [Statement<'a>]) -> RouterBindings<'a> {
    let mut bindings = RouterBindings::default();
    for stmt in stmts {
        let Statement::ImportDeclaration(import) = stmt else {
            continue;
        };
        if import.source.value.as_str() != "next/navigation" {
            continue;
        }
        let Some(specifiers) = &import.specifiers else {
            continue;
        };
        for specifier in specifiers {
            let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier else {
                continue;
            };
            if specifier.imported.name().as_str() == "redirect" {
                bindings.redirects.insert(specifier.local.name.as_str());
            }
        }
    }
    bindings
}

fn register_router_bindings_from_statement<'a>(
    stmt: &'a Statement<'a>,
    bindings: &mut RouterBindings<'a>,
) {
    match stmt {
        Statement::VariableDeclaration(var_decl) => {
            collect_router_bindings_from_var_decl(var_decl, bindings);
        }
        Statement::FunctionDeclaration(func) => {
            remove_shadowed_function_binding(func, bindings);
        }
        Statement::ClassDeclaration(class) => {
            remove_shadowed_class_binding(class, bindings);
        }
        Statement::ForStatement(for_stmt) => match &for_stmt.init {
            Some(ForStatementInit::VariableDeclaration(var_decl))
                if var_decl.kind == VariableDeclarationKind::Var =>
            {
                collect_router_bindings_from_var_decl(var_decl, bindings);
            }
            _ => {}
        },
        Statement::ForInStatement(for_stmt) => {
            collect_for_statement_left_var_bindings(&for_stmt.left, bindings);
        }
        Statement::ForOfStatement(for_stmt) => {
            collect_for_statement_left_var_bindings(&for_stmt.left, bindings);
        }
        Statement::ExportNamedDeclaration(export) => match export.declaration.as_ref() {
            Some(oxc::ast::ast::Declaration::VariableDeclaration(var_decl)) => {
                collect_router_bindings_from_var_decl(var_decl, bindings);
            }
            Some(oxc::ast::ast::Declaration::FunctionDeclaration(func)) => {
                remove_shadowed_function_binding(func, bindings);
            }
            Some(oxc::ast::ast::Declaration::ClassDeclaration(class)) => {
                remove_shadowed_class_binding(class, bindings);
            }
            _ => {}
        },
        _ => {}
    }
}

fn collect_scope_router_bindings<'a>(
    stmts: &'a [Statement<'a>],
    bindings: &mut RouterBindings<'a>,
) {
    for stmt in stmts {
        match stmt {
            Statement::VariableDeclaration(var_decl) => {
                collect_router_bindings_from_var_decl(var_decl, bindings);
            }
            Statement::FunctionDeclaration(func) => {
                remove_shadowed_function_binding(func, bindings);
            }
            Statement::ClassDeclaration(class) => {
                remove_shadowed_class_binding(class, bindings);
            }
            Statement::ExportNamedDeclaration(export) => match export.declaration.as_ref() {
                Some(oxc::ast::ast::Declaration::VariableDeclaration(var_decl)) => {
                    collect_router_bindings_from_var_decl(var_decl, bindings);
                }
                Some(oxc::ast::ast::Declaration::FunctionDeclaration(func)) => {
                    remove_shadowed_function_binding(func, bindings);
                }
                Some(oxc::ast::ast::Declaration::ClassDeclaration(class)) => {
                    remove_shadowed_class_binding(class, bindings);
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn collect_function_scope_var_bindings<'a>(
    stmts: &'a [Statement<'a>],
    bindings: &mut RouterBindings<'a>,
) {
    for stmt in stmts {
        match stmt {
            Statement::VariableDeclaration(var_decl)
                if var_decl.kind == VariableDeclarationKind::Var =>
            {
                collect_router_bindings_from_var_decl(var_decl, bindings);
            }
            Statement::BlockStatement(block) => {
                collect_function_scope_var_bindings(&block.body, bindings);
            }
            Statement::IfStatement(if_stmt) => {
                collect_function_scope_var_bindings(
                    std::slice::from_ref(&if_stmt.consequent),
                    bindings,
                );
                if let Some(alt) = &if_stmt.alternate {
                    collect_function_scope_var_bindings(std::slice::from_ref(alt), bindings);
                }
            }
            Statement::ForStatement(for_stmt) => {
                match &for_stmt.init {
                    Some(ForStatementInit::VariableDeclaration(var_decl))
                        if var_decl.kind == VariableDeclarationKind::Var =>
                    {
                        collect_router_bindings_from_var_decl(var_decl, bindings);
                    }
                    _ => {}
                }
                collect_function_scope_var_bindings(std::slice::from_ref(&for_stmt.body), bindings);
            }
            Statement::ForInStatement(for_stmt) => {
                collect_for_statement_left_var_bindings(&for_stmt.left, bindings);
                collect_function_scope_var_bindings(std::slice::from_ref(&for_stmt.body), bindings);
            }
            Statement::ForOfStatement(for_stmt) => {
                collect_for_statement_left_var_bindings(&for_stmt.left, bindings);
                collect_function_scope_var_bindings(std::slice::from_ref(&for_stmt.body), bindings);
            }
            Statement::WhileStatement(while_stmt) => {
                collect_function_scope_var_bindings(
                    std::slice::from_ref(&while_stmt.body),
                    bindings,
                );
            }
            Statement::DoWhileStatement(do_while_stmt) => {
                collect_function_scope_var_bindings(
                    std::slice::from_ref(&do_while_stmt.body),
                    bindings,
                );
            }
            Statement::SwitchStatement(switch_stmt) => {
                for case in &switch_stmt.cases {
                    collect_function_scope_var_bindings(&case.consequent, bindings);
                }
            }
            Statement::TryStatement(try_stmt) => {
                collect_function_scope_var_bindings(&try_stmt.block.body, bindings);
                if let Some(handler) = &try_stmt.handler {
                    collect_function_scope_var_bindings(&handler.body.body, bindings);
                }
                if let Some(finalizer) = &try_stmt.finalizer {
                    collect_function_scope_var_bindings(&finalizer.body, bindings);
                }
            }
            _ => {}
        }
    }
}

fn collect_router_bindings_for_scope<'a>(
    stmts: &'a [Statement<'a>],
    bindings: &mut RouterBindings<'a>,
) {
    collect_scope_router_bindings(stmts, bindings);
    collect_function_scope_var_bindings(stmts, bindings);
}

fn collect_for_statement_left_var_bindings<'a>(
    left: &'a ForStatementLeft<'a>,
    bindings: &mut RouterBindings<'a>,
) {
    if let ForStatementLeft::VariableDeclaration(var_decl) = left {
        if var_decl.kind == VariableDeclarationKind::Var {
            collect_router_bindings_from_var_decl(var_decl, bindings);
        }
    }
}

fn collect_router_bindings_from_var_decl<'a>(
    var_decl: &'a oxc::ast::ast::VariableDeclaration<'a>,
    bindings: &mut RouterBindings<'a>,
) {
    for decl in &var_decl.declarations {
        remove_shadowed_binding(&decl.id, bindings);
        if decl
            .init
            .as_ref()
            .map(|init| is_use_router_call(init))
            .unwrap_or(false)
        {
            add_router_binding_pattern(&decl.id, bindings);
        }
    }
}

fn add_router_binding_pattern<'a>(
    pattern: &'a BindingPattern<'a>,
    bindings: &mut RouterBindings<'a>,
) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => {
            bindings.objects.insert(id.name.as_str());
        }
        BindingPattern::ObjectPattern(obj) => {
            for prop in &obj.properties {
                let Some(key) = prop.key.static_name() else {
                    continue;
                };
                if !matches!(key.as_ref(), "push" | "replace" | "prefetch") {
                    continue;
                }
                if let Some(name) = router_method_binding_name(&prop.value) {
                    bindings.methods.insert(name);
                }
            }
        }
        _ => {}
    }
}

fn router_method_binding_name<'a>(pattern: &'a BindingPattern<'a>) -> Option<&'a str> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
        BindingPattern::AssignmentPattern(assign) => router_method_binding_name(&assign.left),
        _ => None,
    }
}

fn remove_shadowed_name(name: &str, bindings: &mut RouterBindings<'_>) {
    bindings.objects.remove(name);
    bindings.methods.remove(name);
    bindings.redirects.remove(name);
}

fn remove_shadowed_function_binding(
    func: &oxc::ast::ast::Function,
    bindings: &mut RouterBindings<'_>,
) {
    if let Some(id) = &func.id {
        remove_shadowed_name(id.name.as_str(), bindings);
    }
}

fn remove_shadowed_class_binding(class: &oxc::ast::ast::Class, bindings: &mut RouterBindings<'_>) {
    if let Some(id) = &class.id {
        remove_shadowed_name(id.name.as_str(), bindings);
    }
}

fn remove_shadowed_binding(pattern: &BindingPattern, bindings: &mut RouterBindings<'_>) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => {
            remove_shadowed_name(id.name.as_str(), bindings);
        }
        BindingPattern::ObjectPattern(obj) => {
            for prop in &obj.properties {
                remove_shadowed_binding(&prop.value, bindings);
            }
            if let Some(rest) = &obj.rest {
                remove_shadowed_binding(&rest.argument, bindings);
            }
        }
        BindingPattern::ArrayPattern(arr) => {
            for element in arr.elements.iter().flatten() {
                remove_shadowed_binding(element, bindings);
            }
            if let Some(rest) = &arr.rest {
                remove_shadowed_binding(&rest.argument, bindings);
            }
        }
        BindingPattern::AssignmentPattern(assign) => {
            remove_shadowed_binding(&assign.left, bindings);
        }
    }
}

fn remove_shadowed_parameters(
    params: &oxc::ast::ast::FormalParameters,
    bindings: &mut RouterBindings<'_>,
) {
    for param in &params.items {
        remove_shadowed_binding(&param.pattern, bindings);
    }
    if let Some(rest) = &params.rest {
        remove_shadowed_binding(&rest.rest.argument, bindings);
    }
}

fn is_use_router_call(expr: &Expression) -> bool {
    if let Expression::CallExpression(call) = expr {
        match &call.callee {
            Expression::Identifier(id) => id.name.as_str() == "useRouter",
            other => other
                .as_member_expression()
                .and_then(|m| m.static_property_name())
                .map(|n| n == "useRouter")
                .unwrap_or(false),
        }
    } else {
        false
    }
}

fn collect_from_statement<'a>(
    stmt: &'a Statement<'a>,
    source: &str,
    file: &str,
    router_bindings: &mut RouterBindings<'a>,
    refs: &mut Vec<RouteRef>,
) {
    register_router_bindings_from_statement(stmt, router_bindings);

    match stmt {
        Statement::ExpressionStatement(expr_stmt) => {
            collect_from_expression(&expr_stmt.expression, source, file, router_bindings, refs);
        }
        Statement::ReturnStatement(ret_stmt) => {
            if let Some(expr) = &ret_stmt.argument {
                collect_from_expression(expr, source, file, router_bindings, refs);
            }
        }
        Statement::BlockStatement(block) => {
            let mut scoped_bindings = router_bindings.clone();
            collect_router_bindings_for_scope(&block.body, &mut scoped_bindings);
            for s in &block.body {
                collect_from_statement(s, source, file, &mut scoped_bindings, refs);
            }
        }
        Statement::IfStatement(if_stmt) => {
            collect_from_expression(&if_stmt.test, source, file, router_bindings, refs);
            collect_from_statement(&if_stmt.consequent, source, file, router_bindings, refs);
            if let Some(alt) = &if_stmt.alternate {
                collect_from_statement(alt, source, file, router_bindings, refs);
            }
        }
        Statement::VariableDeclaration(var_decl) => {
            for decl in &var_decl.declarations {
                if let Some(init) = &decl.init {
                    collect_from_expression(init, source, file, router_bindings, refs);
                }
            }
        }
        Statement::FunctionDeclaration(func) => {
            if let Some(body) = &func.body {
                let mut scoped_bindings = router_bindings.clone();
                remove_shadowed_function_binding(func, &mut scoped_bindings);
                remove_shadowed_parameters(&func.params, &mut scoped_bindings);
                collect_router_bindings_for_scope(&body.statements, &mut scoped_bindings);
                for s in &body.statements {
                    collect_from_statement(s, source, file, &mut scoped_bindings, refs);
                }
            }
        }
        Statement::ExportNamedDeclaration(export) => match export.declaration.as_ref() {
            Some(oxc::ast::ast::Declaration::VariableDeclaration(var_decl)) => {
                for init in var_decl
                    .declarations
                    .iter()
                    .filter_map(|decl| decl.init.as_ref())
                {
                    collect_from_expression(init, source, file, router_bindings, refs);
                }
            }
            Some(oxc::ast::ast::Declaration::FunctionDeclaration(func)) => {
                collect_from_function_body(func, source, file, router_bindings, refs);
            }
            _ => {}
        },
        Statement::ExportDefaultDeclaration(export) => match &export.declaration {
            oxc::ast::ast::ExportDefaultDeclarationKind::FunctionDeclaration(func) => {
                collect_from_function_body(func, source, file, router_bindings, refs);
            }
            other => {
                if let Some(expr) = other.as_expression() {
                    collect_from_expression(expr, source, file, router_bindings, refs);
                }
            }
        },
        _ => {}
    }
}

fn collect_from_function_body<'a>(
    func: &'a oxc::ast::ast::Function<'a>,
    source: &str,
    file: &str,
    router_bindings: &mut RouterBindings<'a>,
    refs: &mut Vec<RouteRef>,
) {
    let Some(body) = &func.body else {
        return;
    };
    let mut scoped_bindings = router_bindings.clone();
    remove_shadowed_function_binding(func, &mut scoped_bindings);
    remove_shadowed_parameters(&func.params, &mut scoped_bindings);
    collect_router_bindings_for_scope(&body.statements, &mut scoped_bindings);
    for s in &body.statements {
        collect_from_statement(s, source, file, &mut scoped_bindings, refs);
    }
}

fn collect_from_expression<'a>(
    expr: &'a Expression<'a>,
    source: &str,
    file: &str,
    router_bindings: &mut RouterBindings<'a>,
    refs: &mut Vec<RouteRef>,
) {
    match expr {
        Expression::JSXElement(jsx_elem) => {
            collect_from_jsx_element(jsx_elem, source, file, router_bindings, refs);
        }
        Expression::JSXFragment(frag) => {
            for child in &frag.children {
                collect_from_jsx_child(child, source, file, router_bindings, refs);
            }
        }
        Expression::CallExpression(call) => {
            check_call_for_route_ref(call, source, file, router_bindings, refs);
            collect_from_expression(&call.callee, source, file, router_bindings, refs);
            for arg in &call.arguments {
                collect_from_argument(arg, source, file, router_bindings, refs);
            }
        }
        Expression::ArrowFunctionExpression(arrow) => {
            let mut scoped_bindings = router_bindings.clone();
            remove_shadowed_parameters(&arrow.params, &mut scoped_bindings);
            collect_router_bindings_for_scope(&arrow.body.statements, &mut scoped_bindings);
            for s in &arrow.body.statements {
                collect_from_statement(s, source, file, &mut scoped_bindings, refs);
            }
        }
        Expression::FunctionExpression(func) => {
            collect_from_function_body(func, source, file, router_bindings, refs);
        }
        Expression::ConditionalExpression(cond) => {
            collect_from_expression(&cond.test, source, file, router_bindings, refs);
            collect_from_expression(&cond.consequent, source, file, router_bindings, refs);
            collect_from_expression(&cond.alternate, source, file, router_bindings, refs);
        }
        Expression::LogicalExpression(logical) => {
            collect_from_expression(&logical.left, source, file, router_bindings, refs);
            collect_from_expression(&logical.right, source, file, router_bindings, refs);
        }
        Expression::SequenceExpression(seq) => {
            for e in &seq.expressions {
                collect_from_expression(e, source, file, router_bindings, refs);
            }
        }
        Expression::AssignmentExpression(assign) => {
            collect_from_expression(&assign.right, source, file, router_bindings, refs);
        }
        Expression::ParenthesizedExpression(paren) => {
            collect_from_expression(&paren.expression, source, file, router_bindings, refs);
        }
        Expression::TSAsExpression(ts_as) => {
            collect_from_expression(&ts_as.expression, source, file, router_bindings, refs);
        }
        Expression::TSTypeAssertion(ts_assertion) => {
            collect_from_expression(
                &ts_assertion.expression,
                source,
                file,
                router_bindings,
                refs,
            );
        }
        Expression::TSNonNullExpression(ts_nn) => {
            collect_from_expression(&ts_nn.expression, source, file, router_bindings, refs);
        }
        Expression::TSSatisfiesExpression(ts_sat) => {
            collect_from_expression(&ts_sat.expression, source, file, router_bindings, refs);
        }
        _ => {}
    }
}

fn collect_from_argument<'a>(
    arg: &'a Argument<'a>,
    source: &str,
    file: &str,
    router_bindings: &mut RouterBindings<'a>,
    refs: &mut Vec<RouteRef>,
) {
    match arg {
        Argument::SpreadElement(s) => {
            collect_from_expression(&s.argument, source, file, router_bindings, refs);
        }
        _ => {
            if let Some(expr) = arg.as_expression() {
                collect_from_expression(expr, source, file, router_bindings, refs);
            }
        }
    }
}

fn collect_from_jsx_element<'a>(
    jsx_elem: &'a JSXElement<'a>,
    source: &str,
    file: &str,
    router_bindings: &mut RouterBindings<'a>,
    refs: &mut Vec<RouteRef>,
) {
    for attr_item in &jsx_elem.opening_element.attributes {
        let JSXAttributeItem::Attribute(attr) = attr_item else {
            continue;
        };
        let attr_name = match &attr.name {
            JSXAttributeName::Identifier(id) => id.name.as_str(),
            JSXAttributeName::NamespacedName(_) => continue,
        };

        if attr_name != "href" && attr_name != "to" {
            continue;
        }

        let line = byte_offset_to_line(source, attr.span.start as usize);

        let pattern = match &attr.value {
            Some(JSXAttributeValue::StringLiteral(s)) => Some(s.value.as_str().to_string()),
            Some(JSXAttributeValue::ExpressionContainer(container)) => {
                extract_pattern_from_jsx_expression(&container.expression)
            }
            _ => None,
        }
        .filter(|pattern| !should_skip(pattern));

        if let Some(pattern) = pattern {
            refs.push(RouteRef {
                pattern,
                file: file.to_string(),
                line,
            });
        }
    }

    for child in &jsx_elem.children {
        collect_from_jsx_child(child, source, file, router_bindings, refs);
    }
}

fn collect_from_jsx_child<'a>(
    child: &'a JSXChild<'a>,
    source: &str,
    file: &str,
    router_bindings: &mut RouterBindings<'a>,
    refs: &mut Vec<RouteRef>,
) {
    match child {
        JSXChild::Element(elem) => {
            collect_from_jsx_element(elem, source, file, router_bindings, refs)
        }
        JSXChild::Fragment(frag) => {
            for c in &frag.children {
                collect_from_jsx_child(c, source, file, router_bindings, refs);
            }
        }
        JSXChild::ExpressionContainer(container) => {
            if let Some(expr) = container.expression.as_expression() {
                collect_from_expression(expr, source, file, router_bindings, refs);
            }
        }
        _ => {}
    }
}

fn extract_pattern_from_jsx_expression(jsx_expr: &JSXExpression) -> Option<String> {
    match jsx_expr {
        JSXExpression::EmptyExpression(_) => None,
        _ => jsx_expr
            .as_expression()
            .and_then(extract_pattern_from_expression),
    }
}

fn check_call_for_route_ref(
    call: &oxc::ast::ast::CallExpression,
    source: &str,
    file: &str,
    router_bindings: &RouterBindings<'_>,
    refs: &mut Vec<RouteRef>,
) {
    // Detect router.push('/path') / router.replace('/path') where router is
    // bound to useRouter().
    if let Some(member) = call.callee.as_member_expression() {
        let is_router_method = member
            .static_property_name()
            .is_some_and(|prop| prop == "push" || prop == "replace" || prop == "prefetch");
        if is_router_method {
            if let Expression::Identifier(ident) = member.object() {
                let name = ident.name.as_str();
                if router_bindings.objects.contains(name) {
                    let line = byte_offset_to_line(source, call.span.start as usize);
                    if let Some(pattern) =
                        first_arg_pattern(&call.arguments).filter(|p| !should_skip(p))
                    {
                        refs.push(RouteRef {
                            pattern,
                            file: file.to_string(),
                            line,
                        });
                    }
                }
            }
        }
    }

    if let Expression::Identifier(id) = &call.callee {
        let name = id.name.as_str();
        if router_bindings.redirects.contains(name) || router_bindings.methods.contains(name) {
            let line = byte_offset_to_line(source, call.span.start as usize);
            if let Some(pattern) = first_arg_pattern(&call.arguments).filter(|p| !should_skip(p)) {
                refs.push(RouteRef {
                    pattern,
                    file: file.to_string(),
                    line,
                });
            }
        }
    }

    let is_fetch = match &call.callee {
        Expression::Identifier(id) => id.name.as_str() == "fetch",
        other => other
            .as_member_expression()
            .and_then(|m| m.static_property_name())
            .map(|n| n == "fetch")
            .unwrap_or(false),
    };

    if is_fetch {
        let line = byte_offset_to_line(source, call.span.start as usize);
        if let Some(pattern) = first_arg_pattern(&call.arguments) {
            // Only capture fetch() calls to local absolute paths (starting with '/').
            // External URLs (http/https) are already filtered by should_skip().
            if pattern.starts_with('/') && !should_skip(&pattern) {
                refs.push(RouteRef {
                    pattern,
                    file: file.to_string(),
                    line,
                });
            }
        }
    }
}

fn first_arg_pattern(arguments: &oxc::allocator::Vec<Argument>) -> Option<String> {
    let first = arguments.first()?;
    match first {
        Argument::StringLiteral(s) => Some(s.value.as_str().to_string()),
        Argument::TemplateLiteral(tpl) => Some(normalize_template(tpl)),
        _ => {
            if let Some(expr) = first.as_expression() {
                extract_pattern_from_expression(expr)
            } else {
                None
            }
        }
    }
}

fn extract_pattern_from_expression(expr: &Expression) -> Option<String> {
    match expr {
        Expression::StringLiteral(s) => Some(normalize_next_pathname_pattern(s.value.as_str())),
        Expression::TemplateLiteral(tpl) => Some(normalize_template(tpl)),
        Expression::ObjectExpression(obj) => object_pathname(obj),
        Expression::TSTypeAssertion(ts_assertion) => {
            extract_pattern_from_expression(&ts_assertion.expression)
        }
        _ => None,
    }
}

fn object_pathname(obj: &oxc::ast::ast::ObjectExpression) -> Option<String> {
    for prop in &obj.properties {
        let ObjectPropertyKind::ObjectProperty(prop) = prop else {
            continue;
        };
        let is_pathname = match &prop.key {
            PropertyKey::StaticIdentifier(id) => id.name == "pathname",
            PropertyKey::StringLiteral(s) => s.value == "pathname",
            _ => false,
        };
        if is_pathname {
            return extract_pattern_from_expression(&prop.value);
        }
    }
    None
}

fn normalize_next_pathname_pattern(path: &str) -> String {
    let leading_slash = path.starts_with('/');
    let trailing_slash = path.ends_with('/') && path.len() > 1;
    let segments: Vec<String> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            if segment.starts_with("[[...") && segment.ends_with("]]") {
                "**".to_string()
            } else if segment.starts_with("[...") && segment.ends_with(']') {
                "*".to_string()
            } else if segment.starts_with('[') && segment.ends_with(']') {
                format!(":{}", &segment[1..segment.len() - 1])
            } else {
                segment.to_string()
            }
        })
        .collect();

    let mut normalized = if leading_slash {
        format!("/{}", segments.join("/"))
    } else {
        segments.join("/")
    };
    if trailing_slash {
        normalized.push('/');
    }
    normalized
}

/// Normalize a template literal to a route pattern (replaces `${...}` with `:param`).
pub fn normalize_template(tpl: &TemplateLiteral) -> String {
    let mut result = String::new();
    for (i, quasi) in tpl.quasis.iter().enumerate() {
        if let Some(cooked) = quasi.value.cooked {
            result.push_str(cooked.as_str());
        }
        if i < tpl.expressions.len() {
            result.push_str(":param");
        }
    }
    normalize_next_pathname_pattern(&result)
}

/// Returns true if this reference should be skipped.
pub fn should_skip(pattern: &str) -> bool {
    if pattern.is_empty() {
        return true;
    }
    if pattern.starts_with("http://")
        || pattern.starts_with("https://")
        || pattern.starts_with("//")
    {
        return true;
    }
    if pattern.starts_with('?') || pattern.starts_with('#') {
        return true;
    }
    if pattern.starts_with(":param") {
        return true;
    }
    false
}

#[cfg(test)]
mod tests;
