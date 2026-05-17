use super::{binding_name, AnalysisCollector};
use crate::integration_tests::calls;
use crate::integration_tests::types::{FunctionInfo, ImportBinding, ImportedName};
use oxc_ast::ast::{
    ExportDefaultDeclarationKind, Expression, Function, ImportDeclarationSpecifier,
};
use oxc_span::{GetSpan, Span};

impl AnalysisCollector<'_, '_> {
    pub(super) fn collect_import(
        &mut self,
        specifier: &ImportDeclarationSpecifier<'_>,
        source: &str,
    ) {
        let (local, imported) = match specifier {
            ImportDeclarationSpecifier::ImportDefaultSpecifier(default) => {
                (default.local.name.to_string(), ImportedName::Default)
            }
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(namespace) => {
                (namespace.local.name.to_string(), ImportedName::Namespace)
            }
            ImportDeclarationSpecifier::ImportSpecifier(named) => (
                named.local.name.to_string(),
                ImportedName::Named(named.imported.name().to_string()),
            ),
        };
        self.result.imports.insert(
            local,
            ImportBinding {
                source: source.to_string(),
                imported,
            },
        );
    }

    pub(super) fn collect_export_declaration(
        &mut self,
        declaration: &oxc_ast::ast::Declaration<'_>,
    ) {
        match declaration {
            oxc_ast::ast::Declaration::FunctionDeclaration(function) => {
                if let Some(id) = &function.id {
                    self.collect_function(
                        id.name.as_str(),
                        function.span(),
                        function.body_span(),
                        false,
                    );
                    self.result
                        .exports
                        .insert(id.name.to_string(), id.name.to_string());
                }
            }
            oxc_ast::ast::Declaration::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    let Some(name) = binding_name(&declarator.id) else {
                        continue;
                    };
                    self.collect_init_function(name, declarator.init.as_ref());
                    self.result
                        .exports
                        .insert(name.to_string(), name.to_string());
                }
            }
            _ => {}
        }
    }

    pub(super) fn collect_default_export(
        &mut self,
        export: &oxc_ast::ast::ExportDefaultDeclaration<'_>,
    ) {
        match &export.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                let name = function
                    .id
                    .as_ref()
                    .map(|id| id.name.as_str())
                    .unwrap_or("default");
                self.collect_function(name, function.span(), function.body_span(), false);
                self.result
                    .exports
                    .insert("default".to_string(), name.to_string());
            }
            ExportDefaultDeclarationKind::Identifier(identifier) => {
                self.result
                    .exports
                    .insert("default".to_string(), identifier.name.to_string());
            }
            ExportDefaultDeclarationKind::ArrowFunctionExpression(arrow) => {
                self.collect_function(
                    "default",
                    arrow.span(),
                    Some(arrow.body.span()),
                    arrow.expression,
                );
                self.result
                    .exports
                    .insert("default".to_string(), "default".to_string());
            }
            _ => {}
        }
    }

    pub(super) fn collect_init_function(&mut self, name: &str, init: Option<&Expression<'_>>) {
        match init {
            Some(Expression::ArrowFunctionExpression(arrow)) => {
                self.collect_function(
                    name,
                    arrow.span(),
                    Some(arrow.body.span()),
                    arrow.expression,
                );
            }
            Some(Expression::FunctionExpression(function)) => {
                self.collect_function(name, function.span(), function.body_span(), false);
            }
            _ => {}
        }
    }

    pub(super) fn collect_function(
        &mut self,
        name: &str,
        function_span: Span,
        body_span: Option<Span>,
        expression: bool,
    ) {
        self.result.functions.insert(
            name.to_string(),
            FunctionInfo {
                integration: calls::integration_annotation_before(self.source, function_span),
                calls: body_span
                    .map(|span| calls::collect_calls_in_span(self.source, span, expression))
                    .unwrap_or_default(),
            },
        );
    }
}

pub(super) trait FunctionBodySpan {
    fn body_span(&self) -> Option<Span>;
}

impl FunctionBodySpan for Function<'_> {
    fn body_span(&self) -> Option<Span> {
        self.body.as_ref().map(|body| body.span())
    }
}
