use crate::codebase::ts_source::byte_offset_to_line;
use anyhow::Result;
use oxc::allocator::Allocator;
use oxc::ast::ast::{BindingPattern, Declaration, ExportDefaultDeclarationKind, Statement};
use oxc::parser::Parser;
use oxc::span::SourceType;

/// The kind of a top-level export.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportKind {
    Function,
    Class,
    Const,
    Let,
    Var,
    TypeAlias,
    Interface,
    Enum,
    Default,
    /// Re-export: `export { name } from 'source'` or `export * from 'source'`.
    ReExport {
        source: String,
        /// The imported symbol name in the source module. `"*"` for star re-exports.
        imported: String,
    },
}

/// A top-level exported symbol.
#[derive(Debug, Clone, PartialEq)]
pub struct Export {
    pub name: String,
    pub kind: ExportKind,
    pub line: u32,
}

/// A named import statement.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedImport {
    /// The module specifier (e.g. `"./foo.mts"`, `"@utils/helpers"`).
    pub source: String,
    /// The name as exported from the source module.
    pub imported: String,
    /// The local binding name (may differ from `imported` when aliased).
    pub local: String,
    pub line: u32,
    pub is_type_only: bool,
}

/// All top-level exports and named imports extracted from a file.
#[derive(Debug, Default, Clone)]
pub struct FileSymbols {
    pub exports: Vec<Export>,
    pub imports: Vec<NamedImport>,
}

/// Extract top-level exports and named imports from TypeScript/TSX source.
pub fn extract_symbols(source: &str, is_tsx: bool) -> Result<FileSymbols> {
    let allocator = Allocator::default();
    let source_type = if is_tsx {
        SourceType::tsx()
    } else {
        SourceType::ts()
    };
    let ret = Parser::new(&allocator, source, source_type).parse();

    let mut symbols = FileSymbols::default();

    for stmt in &ret.program.body {
        process_statement(stmt, source, &mut symbols);
    }

    Ok(symbols)
}

fn process_statement(stmt: &Statement, source: &str, out: &mut FileSymbols) {
    match stmt {
        Statement::ImportDeclaration(import) => {
            let src = import.source.value.as_str().to_string();
            let is_type = import.import_kind.is_type();
            if let Some(specifiers) = &import.specifiers {
                for spec in specifiers {
                    match spec {
                        oxc::ast::ast::ImportDeclarationSpecifier::ImportSpecifier(s) => {
                            let imported = s.imported.name().to_string();
                            let local = s.local.name.as_str().to_string();
                            let line = byte_offset_to_line(source, s.span.start as usize);
                            out.imports.push(NamedImport {
                                source: src.clone(),
                                imported,
                                local,
                                line,
                                is_type_only: is_type || s.import_kind.is_type(),
                            });
                        }
                        oxc::ast::ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                            let local = s.local.name.as_str().to_string();
                            let line = byte_offset_to_line(source, s.span.start as usize);
                            out.imports.push(NamedImport {
                                source: src.clone(),
                                imported: "*".to_string(),
                                local,
                                line,
                                is_type_only: is_type,
                            });
                        }
                        oxc::ast::ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                            let local = s.local.name.as_str().to_string();
                            let line = byte_offset_to_line(source, s.span.start as usize);
                            out.imports.push(NamedImport {
                                source: src.clone(),
                                imported: "default".to_string(),
                                local,
                                line,
                                is_type_only: is_type,
                            });
                        }
                    }
                }
            }
        }

        Statement::ExportNamedDeclaration(export) => {
            let line = byte_offset_to_line(source, export.span.start as usize);

            // Re-export with source: `export { X } from './y'`
            if let Some(src) = &export.source {
                let source_str = src.value.as_str().to_string();
                for spec in &export.specifiers {
                    let imported = spec.local.name().to_string();
                    let name = spec.exported.name().to_string();
                    out.exports.push(Export {
                        name,
                        kind: ExportKind::ReExport {
                            source: source_str.clone(),
                            imported,
                        },
                        line,
                    });
                }
                return;
            }

            // Inline declaration: `export function foo()`, `export const x = ...`
            if let Some(decl) = &export.declaration {
                match decl {
                    Declaration::FunctionDeclaration(func) => {
                        if let Some(id) = &func.id {
                            out.exports.push(Export {
                                name: id.name.as_str().to_string(),
                                kind: ExportKind::Function,
                                line,
                            });
                        }
                    }
                    Declaration::ClassDeclaration(cls) => {
                        if let Some(id) = &cls.id {
                            out.exports.push(Export {
                                name: id.name.as_str().to_string(),
                                kind: ExportKind::Class,
                                line,
                            });
                        }
                    }
                    Declaration::VariableDeclaration(var) => {
                        let kind = match var.kind {
                            oxc::ast::ast::VariableDeclarationKind::Const => ExportKind::Const,
                            oxc::ast::ast::VariableDeclarationKind::Let => ExportKind::Let,
                            oxc::ast::ast::VariableDeclarationKind::Var => ExportKind::Var,
                            _ => ExportKind::Const, // using/await using treated as const
                        };
                        for decl in &var.declarations {
                            collect_binding_names(&decl.id, kind.clone(), line, out);
                        }
                    }
                    Declaration::TSTypeAliasDeclaration(ta) => {
                        out.exports.push(Export {
                            name: ta.id.name.as_str().to_string(),
                            kind: ExportKind::TypeAlias,
                            line,
                        });
                    }
                    Declaration::TSInterfaceDeclaration(iface) => {
                        out.exports.push(Export {
                            name: iface.id.name.as_str().to_string(),
                            kind: ExportKind::Interface,
                            line,
                        });
                    }
                    Declaration::TSEnumDeclaration(en) => {
                        out.exports.push(Export {
                            name: en.id.name.as_str().to_string(),
                            kind: ExportKind::Enum,
                            line,
                        });
                    }
                    _ => {}
                }
                return;
            }

            // Specifier exports without source: `export { a, b }` (local re-bindings)
            for spec in &export.specifiers {
                let name = spec.exported.name().to_string();
                out.exports.push(Export {
                    name,
                    kind: ExportKind::Const,
                    line,
                });
            }
        }

        Statement::ExportDefaultDeclaration(export) => {
            let line = byte_offset_to_line(source, export.span.start as usize);
            let name = match &export.declaration {
                ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                    f.id.as_ref()
                        .map(|id| id.name.as_str().to_string())
                        .unwrap_or_else(|| "default".to_string())
                }
                ExportDefaultDeclarationKind::ClassDeclaration(c) => {
                    c.id.as_ref()
                        .map(|id| id.name.as_str().to_string())
                        .unwrap_or_else(|| "default".to_string())
                }
                ExportDefaultDeclarationKind::TSInterfaceDeclaration(i) => {
                    i.id.name.as_str().to_string()
                }
                ExportDefaultDeclarationKind::Identifier(id) => id.name.as_str().to_string(),
                _ => "default".to_string(),
            };
            out.exports.push(Export {
                name,
                kind: ExportKind::Default,
                line,
            });
        }

        Statement::ExportAllDeclaration(export) => {
            let source_str = export.source.value.as_str().to_string();
            let line = byte_offset_to_line(source, export.span.start as usize);
            out.exports.push(Export {
                name: "*".to_string(),
                kind: ExportKind::ReExport {
                    source: source_str,
                    imported: "*".to_string(),
                },
                line,
            });
        }

        _ => {}
    }
}

fn collect_binding_names(pat: &BindingPattern, kind: ExportKind, line: u32, out: &mut FileSymbols) {
    match pat {
        BindingPattern::BindingIdentifier(id) => {
            out.exports.push(Export {
                name: id.name.as_str().to_string(),
                kind,
                line,
            });
        }
        BindingPattern::ObjectPattern(obj) => {
            for prop in &obj.properties {
                collect_binding_names(&prop.value, kind.clone(), line, out);
            }
        }
        BindingPattern::ArrayPattern(arr) => {
            for elem in arr.elements.iter().flatten() {
                collect_binding_names(elem, kind.clone(), line, out);
            }
        }
        BindingPattern::AssignmentPattern(ap) => {
            collect_binding_names(&ap.left, kind, line, out);
        }
    }
}

#[cfg(test)]
mod tests;
