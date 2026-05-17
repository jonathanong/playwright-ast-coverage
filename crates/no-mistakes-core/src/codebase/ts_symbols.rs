use crate::codebase::ts_source::byte_offset_to_line;
use anyhow::{bail, Result};
use oxc::allocator::Allocator;
use oxc::ast::ast::{
    BindingPattern, Declaration, ExportDefaultDeclarationKind, Program, Statement,
};
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
    pub is_type_only: bool,
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
    if ret.panicked {
        let detail = ret
            .errors
            .first()
            .map(|err| format!("{err:?}"))
            .unwrap_or("unknown error (parser panicked)".to_string());
        bail!("failed to parse TypeScript source: {detail}");
    }

    Ok(extract_symbols_from_program(&ret.program, source))
}

pub fn extract_symbols_from_program(program: &Program<'_>, source: &str) -> FileSymbols {
    let mut symbols = FileSymbols::default();
    for stmt in &program.body {
        process_statement(stmt, source, &mut symbols);
    }
    symbols
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
            let export_is_type = export.export_kind.is_type();

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
                        is_type_only: export_is_type || spec.export_kind.is_type(),
                    });
                }
                return;
            }

            // Inline declaration: `export function foo()`, `export const x = ...`
            if let Some(decl) = &export.declaration {
                match decl {
                    Declaration::FunctionDeclaration(func) => {
                        push_export_if_named(
                            out,
                            func.id.as_ref().map(|id| id.name.as_str()),
                            ExportKind::Function,
                            line,
                            false,
                        );
                    }
                    Declaration::ClassDeclaration(cls) => {
                        push_export_if_named(
                            out,
                            cls.id.as_ref().map(|id| id.name.as_str()),
                            ExportKind::Class,
                            line,
                            false,
                        );
                    }
                    Declaration::VariableDeclaration(var) => {
                        let kind = match var.kind {
                            oxc::ast::ast::VariableDeclarationKind::Const
                            | oxc::ast::ast::VariableDeclarationKind::Using
                            | oxc::ast::ast::VariableDeclarationKind::AwaitUsing => {
                                ExportKind::Const
                            }
                            oxc::ast::ast::VariableDeclarationKind::Let => ExportKind::Let,
                            oxc::ast::ast::VariableDeclarationKind::Var => ExportKind::Var,
                        };
                        for decl in &var.declarations {
                            collect_binding_names(&decl.id, kind.clone(), line, false, out);
                        }
                    }
                    Declaration::TSTypeAliasDeclaration(ta) => {
                        out.exports.push(Export {
                            name: ta.id.name.as_str().to_string(),
                            kind: ExportKind::TypeAlias,
                            line,
                            is_type_only: true,
                        });
                    }
                    Declaration::TSInterfaceDeclaration(iface) => {
                        out.exports.push(Export {
                            name: iface.id.name.as_str().to_string(),
                            kind: ExportKind::Interface,
                            line,
                            is_type_only: true,
                        });
                    }
                    Declaration::TSEnumDeclaration(en) => {
                        out.exports.push(Export {
                            name: en.id.name.as_str().to_string(),
                            kind: ExportKind::Enum,
                            line,
                            is_type_only: false,
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
                    is_type_only: export_is_type || spec.export_kind.is_type(),
                });
            }
        }

        Statement::ExportDefaultDeclaration(export) => {
            let line = byte_offset_to_line(source, export.span.start as usize);
            let is_type_only = matches!(
                &export.declaration,
                ExportDefaultDeclarationKind::TSInterfaceDeclaration(_)
            );
            let name = match &export.declaration {
                ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                    default_export_name(f.id.as_ref().map(|id| id.name.as_str()))
                }
                ExportDefaultDeclarationKind::ClassDeclaration(c) => {
                    default_export_name(c.id.as_ref().map(|id| id.name.as_str()))
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
                is_type_only,
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
                is_type_only: export.export_kind.is_type(),
            });
        }

        _ => {}
    }
}

fn push_export_if_named(
    out: &mut FileSymbols,
    name: Option<&str>,
    kind: ExportKind,
    line: u32,
    is_type_only: bool,
) {
    if let Some(name) = name {
        out.exports.push(Export {
            name: name.to_string(),
            kind,
            line,
            is_type_only,
        });
    }
}

fn default_export_name(name: Option<&str>) -> String {
    name.unwrap_or("default").to_string()
}

fn collect_binding_names(
    pat: &BindingPattern,
    kind: ExportKind,
    line: u32,
    is_type_only: bool,
    out: &mut FileSymbols,
) {
    match pat {
        BindingPattern::BindingIdentifier(id) => {
            out.exports.push(Export {
                name: id.name.as_str().to_string(),
                kind,
                line,
                is_type_only,
            });
        }
        BindingPattern::ObjectPattern(obj) => {
            for prop in &obj.properties {
                collect_binding_names(&prop.value, kind.clone(), line, is_type_only, out);
            }
        }
        BindingPattern::ArrayPattern(arr) => {
            for elem in arr.elements.iter().flatten() {
                collect_binding_names(elem, kind.clone(), line, is_type_only, out);
            }
        }
        BindingPattern::AssignmentPattern(ap) => {
            collect_binding_names(&ap.left, kind, line, is_type_only, out);
        }
    }
}

#[cfg(test)]
mod tests;
