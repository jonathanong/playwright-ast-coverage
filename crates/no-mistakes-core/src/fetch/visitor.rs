use crate::fetch::cache_opts::cache_wrapper_name;
use crate::fetch::types::{CacheKind, FetchOccurrence};
use crate::fetch::visit_helpers::{enter_cache_wrapper, leave_cache_wrapper, try_extract_fetch};
use oxc_ast::ast::{CallExpression, ImportDeclarationSpecifier};
use oxc_ast_visit::{walk, Visit};
use oxc_span::Span;
use std::collections::HashSet;

pub struct FetchVisitor<'a> {
    pub source: &'a str,
    pub file: String,
    pub fetches: Vec<FetchOccurrence>,
    pub is_client: bool,
    pub is_route_handler: bool,
    pub cached_function: Option<String>,
    pub cached_kind: Option<CacheKind>,
    pub fetch_scope_stack: Vec<FetchScope>,
    pub in_var_declaration: bool,
    pub component_span: Option<Span>,
}

#[derive(Default)]
pub struct FetchScope {
    pub shadowed_identifiers: HashSet<String>,
    pub tracks_var_bindings: bool,
}

impl<'a> FetchVisitor<'a> {
    pub fn new(source: &'a str, file: &str, is_client: bool, is_route_handler: bool) -> Self {
        Self {
            source,
            file: file.to_string(),
            fetches: Vec::new(),
            is_client,
            is_route_handler,
            cached_function: None,
            cached_kind: None,
            fetch_scope_stack: vec![FetchScope {
                shadowed_identifiers: HashSet::new(),
                tracks_var_bindings: true,
            }],
            in_var_declaration: false,
            component_span: None,
        }
    }

    pub fn enter_fetch_scope(&mut self, tracks_var_bindings: bool) {
        self.fetch_scope_stack.push(FetchScope {
            shadowed_identifiers: HashSet::new(),
            tracks_var_bindings,
        });
    }

    pub fn leave_fetch_scope(&mut self) {
        self.fetch_scope_stack.pop();
    }

    pub fn mark_fetch_shadowed(&mut self) {
        let scope = self
            .fetch_scope_stack
            .last_mut()
            .expect("fetch_scope_stack is never empty");
        scope.shadowed_identifiers.insert("fetch".to_string());
    }

    #[inline(never)]
    pub fn mark_identifier_shadowed_in_var_scope(&mut self, name: &str) {
        for scope in self.fetch_scope_stack.iter_mut().rev() {
            if scope.tracks_var_bindings {
                scope.shadowed_identifiers.insert(name.to_string());
                return;
            }
        }

        let scope = self
            .fetch_scope_stack
            .last_mut()
            .expect("fetch_scope_stack is never empty");
        scope.shadowed_identifiers.insert(name.to_string());
    }

    pub fn mark_identifier_shadowed(&mut self, name: &str) {
        let scope = self
            .fetch_scope_stack
            .last_mut()
            .expect("fetch_scope_stack is never empty");
        scope.shadowed_identifiers.insert(name.to_string());
    }

    pub fn is_fetch_shadowed(&self) -> bool {
        self.fetch_scope_stack
            .iter()
            .any(|scope| scope.shadowed_identifiers.contains("fetch"))
    }
}

impl<'a> Visit<'a> for FetchVisitor<'a> {
    fn visit_binding_identifier(&mut self, ident: &oxc_ast::ast::BindingIdentifier<'a>) {
        if self.in_var_declaration {
            self.mark_identifier_shadowed_in_var_scope(ident.name.as_ref());
        } else {
            self.mark_identifier_shadowed(ident.name.as_ref());
        }
        if ident.name.as_ref() == "fetch" {
            self.mark_fetch_shadowed();
        }
        walk::walk_binding_identifier(self, ident);
    }

    fn visit_import_declaration(&mut self, import: &oxc_ast::ast::ImportDeclaration<'a>) {
        if let Some(specifiers) = import.specifiers.as_ref() {
            for specifier in specifiers {
                match specifier {
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(default_import) => {
                        self.mark_identifier_shadowed(default_import.local.name.as_ref());
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(namespace_import) => {
                        self.mark_identifier_shadowed(namespace_import.local.name.as_ref());
                    }
                    ImportDeclarationSpecifier::ImportSpecifier(import_specifier) => {
                        self.mark_identifier_shadowed(import_specifier.local.name.as_ref());
                    }
                }
            }
        }
        walk::walk_import_declaration(self, import);
    }

    fn visit_function(
        &mut self,
        function: &oxc_ast::ast::Function<'a>,
        flags: oxc_syntax::scope::ScopeFlags,
    ) {
        if matches!(
            function.r#type,
            oxc_ast::ast::FunctionType::FunctionDeclaration
                | oxc_ast::ast::FunctionType::TSDeclareFunction
        ) {
            if let Some(id) = &function.id {
                self.mark_identifier_shadowed_in_var_scope(id.name.as_ref());
            }
        }
        self.enter_fetch_scope(true);
        walk::walk_function(self, function, flags);
        self.leave_fetch_scope();
    }

    fn visit_arrow_function_expression(
        &mut self,
        arrow: &oxc_ast::ast::ArrowFunctionExpression<'a>,
    ) {
        self.enter_fetch_scope(false);
        walk::walk_arrow_function_expression(self, arrow);
        self.leave_fetch_scope();
    }

    fn visit_catch_clause(&mut self, catch_clause: &oxc_ast::ast::CatchClause<'a>) {
        self.enter_fetch_scope(false);
        walk::walk_catch_clause(self, catch_clause);
        self.leave_fetch_scope();
    }

    fn visit_variable_declaration(&mut self, declaration: &oxc_ast::ast::VariableDeclaration<'a>) {
        let previous_in_var_declaration = self.in_var_declaration;
        self.in_var_declaration = declaration.kind == oxc_ast::ast::VariableDeclarationKind::Var;
        walk::walk_variable_declaration(self, declaration);
        self.in_var_declaration = previous_in_var_declaration;
    }

    fn visit_block_statement(&mut self, block: &oxc_ast::ast::BlockStatement<'a>) {
        self.enter_fetch_scope(false);
        walk::walk_block_statement(self, block);
        self.leave_fetch_scope();
    }

    fn visit_call_expression(&mut self, expr: &CallExpression<'a>) {
        if cache_wrapper_name(expr).is_some() {
            let (prev_fn, prev_kind) = enter_cache_wrapper(expr, self);
            walk::walk_call_expression(self, expr);
            leave_cache_wrapper(self, prev_fn, prev_kind);
            return;
        }

        let in_scope = self
            .component_span
            .map(|s| expr.span.start >= s.start && expr.span.end <= s.end)
            .unwrap_or(true);

        if in_scope {
            if let Some(occurrence) = try_extract_fetch(expr, self) {
                self.fetches.push(occurrence);
            }
        }
        walk::walk_call_expression(self, expr);
    }
}

#[cfg(test)]
mod tests;
