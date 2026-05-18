use anyhow::Result;
use oxc::allocator::Allocator;
use oxc::ast::ast::{
    Argument, CallExpression, ExportAllDeclaration, ExportNamedDeclaration, ExportSpecifier,
    Expression, ImportDeclaration, ImportDeclarationSpecifier, ImportExpression, Program,
    TSImportType,
};
use oxc::ast_visit::{walk, Visit};
use oxc::parser::Parser;
use oxc::span::SourceType;
use std::path::Path;

/// The syntactic import form that produced an extracted module specifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImportKind {
    /// Static value import/re-export, including side-effect imports.
    Static,
    /// Type-only import/re-export or TypeScript `import("...")` type reference.
    Type,
    /// Runtime dynamic `import("...")`.
    Dynamic,
    /// CommonJS `require("...")` call.
    Require,
}

/// An extracted import specifier with syntax metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedImport {
    pub specifier: String,
    pub kind: ImportKind,
}

/// Holds parser configuration for TypeScript or TSX extraction.
pub struct ImportExtractor {
    is_tsx: bool,
}

impl ImportExtractor {
    pub fn for_typescript() -> Result<Self> {
        Ok(Self { is_tsx: false })
    }

    pub fn for_tsx() -> Result<Self> {
        Ok(Self { is_tsx: true })
    }

    /// Extract import/export specifier strings from `source`, tagging each
    /// with the syntax form that created the dependency.
    pub fn extract(&self, source: &str) -> Result<Vec<ExtractedImport>> {
        let allocator = Allocator::default();
        let source_type = if self.is_tsx {
            SourceType::tsx()
        } else {
            SourceType::ts()
        };
        let ret = Parser::new(&allocator, source, source_type).parse();

        Ok(extract_imports_from_program(&ret.program))
    }
}

pub fn extract_imports_from_program<'a>(program: &Program<'a>) -> Vec<ExtractedImport> {
    let mut collector = ImportCollector::default();
    collector.visit_program(program);
    collector.imports
}

#[derive(Default)]
struct ImportCollector {
    imports: Vec<ExtractedImport>,
}

impl ImportCollector {
    fn push(&mut self, specifier: &str, kind: ImportKind) {
        if !specifier.is_empty() {
            self.imports.push(ExtractedImport {
                specifier: specifier.to_string(),
                kind,
            });
        }
    }
}

impl<'a> Visit<'a> for ImportCollector {
    fn visit_import_declaration(&mut self, import: &ImportDeclaration<'a>) {
        let kind = import_declaration_kind(import);
        self.push(import.source.value.as_str(), kind);
    }

    fn visit_export_named_declaration(&mut self, export: &ExportNamedDeclaration<'a>) {
        if let Some(source) = &export.source {
            let kind = export_named_declaration_kind(export);
            self.push(source.value.as_str(), kind);
        }
        walk::walk_export_named_declaration(self, export);
    }

    fn visit_export_all_declaration(&mut self, export: &ExportAllDeclaration<'a>) {
        let kind = if export.export_kind.is_type() {
            ImportKind::Type
        } else {
            ImportKind::Static
        };
        self.push(export.source.value.as_str(), kind);
    }

    fn visit_import_expression(&mut self, import: &ImportExpression<'a>) {
        if let Some(specifier) = string_literal_expr(&import.source) {
            self.push(specifier, ImportKind::Dynamic);
        }
        walk::walk_import_expression(self, import);
    }

    fn visit_ts_import_type(&mut self, import: &TSImportType<'a>) {
        self.push(import.source.value.as_str(), ImportKind::Type);
        walk::walk_ts_import_type(self, import);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if is_require_callee(&call.callee) {
            if let Some(first) = call.arguments.first() {
                if let Some(specifier) = string_literal_argument(first) {
                    self.push(specifier, ImportKind::Require);
                }
            }
        }
        walk::walk_call_expression(self, call);
    }
}

fn import_declaration_kind(import: &ImportDeclaration<'_>) -> ImportKind {
    if import.import_kind.is_type()
        || all_named_specifiers_are_type(import.specifiers.as_deref().map(|v| &**v))
    {
        ImportKind::Type
    } else {
        ImportKind::Static
    }
}

fn export_named_declaration_kind(export: &ExportNamedDeclaration<'_>) -> ImportKind {
    if export.export_kind.is_type() || all_export_specifiers_are_type(&export.specifiers) {
        ImportKind::Type
    } else {
        ImportKind::Static
    }
}

fn all_named_specifiers_are_type(specifiers: Option<&[ImportDeclarationSpecifier<'_>]>) -> bool {
    let Some(specifiers) = specifiers else {
        return false;
    };
    !specifiers.is_empty()
        && specifiers.iter().all(|spec| {
            matches!(
                spec,
                ImportDeclarationSpecifier::ImportSpecifier(s) if s.import_kind.is_type()
            )
        })
}

fn all_export_specifiers_are_type(specifiers: &[ExportSpecifier<'_>]) -> bool {
    !specifiers.is_empty() && specifiers.iter().all(|s| s.export_kind.is_type())
}

fn is_require_callee(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::Identifier(ident) if ident.name == "require")
}

fn string_literal_expr<'a>(expr: &'a Expression<'a>) -> Option<&'a str> {
    match expr {
        Expression::StringLiteral(s) => Some(s.value.as_str()),
        _ => None,
    }
}

fn string_literal_argument<'a>(arg: &'a Argument<'a>) -> Option<&'a str> {
    match arg {
        Argument::StringLiteral(s) => Some(s.value.as_str()),
        _ => None,
    }
}

/// Returns `true` for `.tsx` / `.jsx` files (which need the TSX grammar).
pub fn is_tsx_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("tsx" | "jsx")
    )
}

/// Returns `true` for any TypeScript/JavaScript source file we should index.
pub fn is_indexable(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("ts" | "mts" | "tsx" | "cts" | "js" | "mjs" | "jsx" | "cjs")
    )
}

#[cfg(test)]
mod tests;
