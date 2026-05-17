mod bindings;
mod helpers;
mod literals;
mod records;

use crate::ast;
use crate::server_routes::model::FileFacts;
use oxc_ast::ast::{
    CallExpression, ExportDefaultDeclarationKind, Expression, ImportDeclarationSpecifier,
    ModuleExportName,
};
use oxc_ast_visit::{walk, Visit};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub(super) const VERBS: &[&str] = &[
    "get", "post", "put", "patch", "delete", "del", "head", "options", "all",
];

pub(crate) fn extract_file(path: &Path) -> anyhow::Result<FileFacts> {
    let source = std::fs::read_to_string(path)?;
    ast::with_program(path, &source, |program, _| {
        let mut visitor = ServerRouteVisitor::new(path, &source);
        visitor.visit_program(program);
        visitor.facts
    })
}

pub(super) struct ServerRouteVisitor<'a> {
    pub(super) path: &'a Path,
    pub(super) source: &'a str,
    pub(super) facts: FileFacts,
    pub(super) const_strings: HashMap<String, String>,
    pub(super) express_names: HashSet<String>,
    pub(super) hono_names: HashSet<String>,
    pub(super) koa_router_names: HashSet<String>,
    pub(super) path_match_names: HashSet<String>,
    pub(super) api_server_names: HashSet<String>,
}

impl<'a> Visit<'a> for ServerRouteVisitor<'a> {
    fn visit_import_declaration(&mut self, import: &oxc_ast::ast::ImportDeclaration<'a>) {
        let source = import.source.value.as_str().to_string();
        if let Some(specifiers) = &import.specifiers {
            for specifier in specifiers {
                self.record_import(&source, specifier);
            }
        }
        walk::walk_import_declaration(self, import);
    }

    fn visit_variable_declarator(&mut self, decl: &oxc_ast::ast::VariableDeclarator<'a>) {
        let Some(name) = helpers::binding_name(&decl.id) else {
            walk::walk_variable_declarator(self, decl);
            return;
        };
        if let Some(init) = &decl.init {
            if let Some(value) = const_string(init) {
                self.const_strings.insert(name.clone(), value);
            }
            if let Some(binding) = self.binding_from_expr(init) {
                self.facts.bindings.insert(name, binding);
            }
        }
        walk::walk_variable_declarator(self, decl);
    }

    fn visit_export_named_declaration(
        &mut self,
        export: &oxc_ast::ast::ExportNamedDeclaration<'a>,
    ) {
        if let Some(oxc_ast::ast::Declaration::VariableDeclaration(var_decl)) = &export.declaration
        {
            for decl in &var_decl.declarations {
                if let Some(name) = helpers::binding_name(&decl.id) {
                    self.facts.exports.insert(name.clone(), name);
                }
            }
        }
        for specifier in &export.specifiers {
            let exported = module_export_name(&specifier.exported);
            let local = module_export_name(&specifier.local);
            self.facts.exports.insert(exported, local);
        }
        walk::walk_export_named_declaration(self, export);
    }

    fn visit_export_default_declaration(
        &mut self,
        export: &oxc_ast::ast::ExportDefaultDeclaration<'a>,
    ) {
        let local =
            default_export_name(&export.declaration).unwrap_or_else(|| "default".to_string());
        self.facts.exports.insert("default".to_string(), local);
        walk::walk_export_default_declaration(self, export);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        self.record_call(call);
        walk::walk_call_expression(self, call);
    }
}

fn module_export_name(name: &ModuleExportName<'_>) -> String {
    match name {
        ModuleExportName::IdentifierName(id) => id.name.to_string(),
        ModuleExportName::IdentifierReference(id) => id.name.to_string(),
        ModuleExportName::StringLiteral(value) => value.value.to_string(),
    }
}

fn default_export_name(decl: &ExportDefaultDeclarationKind<'_>) -> Option<String> {
    match decl {
        ExportDefaultDeclarationKind::Identifier(id) => Some(id.name.to_string()),
        _ => None,
    }
}

fn const_string(expr: &Expression<'_>) -> Option<String> {
    match expr {
        Expression::StringLiteral(value) => Some(value.value.as_str().to_string()),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => Some(
            template
                .quasis
                .iter()
                .filter_map(|quasi| quasi.value.cooked.as_deref())
                .collect::<Vec<_>>()
                .join(""),
        ),
        _ => None,
    }
}

impl<'a> ServerRouteVisitor<'a> {
    fn new(path: &'a Path, source: &'a str) -> Self {
        Self {
            path,
            source,
            facts: FileFacts::default(),
            const_strings: HashMap::new(),
            express_names: HashSet::new(),
            hono_names: HashSet::new(),
            koa_router_names: HashSet::new(),
            path_match_names: HashSet::new(),
            api_server_names: HashSet::new(),
        }
    }
}

pub(super) fn import_names(specifier: &ImportDeclarationSpecifier<'_>) -> (String, String) {
    match specifier {
        ImportDeclarationSpecifier::ImportDefaultSpecifier(spec) => {
            (spec.local.name.to_string(), "default".to_string())
        }
        ImportDeclarationSpecifier::ImportNamespaceSpecifier(spec) => {
            (spec.local.name.to_string(), "*".to_string())
        }
        ImportDeclarationSpecifier::ImportSpecifier(spec) => (
            spec.local.name.to_string(),
            module_export_name(&spec.imported),
        ),
    }
}

#[cfg(test)]
mod tests;
