use crate::ast;
use crate::fetch::import_shape::{is_runtime_export, is_runtime_import};
use crate::fetch::resolve::resolve_import;
use anyhow::Result;
use oxc_ast::ast::{ImportDeclarationSpecifier, ImportOrExportKind, Statement};
use oxc_ast_visit::{walk, Visit};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub fn collect_imports(
    path: &Path,
    import_cache: &mut HashMap<PathBuf, Vec<PathBuf>>,
) -> Result<Vec<PathBuf>> {
    let abs_path = path.canonicalize()?;
    if let Some(cached_imports) = import_cache.get(&abs_path) {
        return Ok(cached_imports.clone());
    }

    let source = std::fs::read_to_string(&abs_path)?;
    let mut imports = Vec::new();
    ast::with_program(path, &source, |program, _source| -> Result<()> {
        imports = collect_imports_from_program(&abs_path, program, import_cache)?;
        Ok(())
    })??;
    Ok(imports)
}

#[derive(Default)]
pub struct IdentifierReferenceCollector {
    pub identifiers: HashSet<String>,
}

impl<'a> Visit<'a> for IdentifierReferenceCollector {
    fn visit_identifier_reference(&mut self, it: &oxc_ast::ast::IdentifierReference<'a>) {
        self.identifiers.insert(it.name.to_string());
        walk::walk_identifier_reference(self, it);
    }
}

pub fn collect_identifier_references(program: &oxc_ast::ast::Program<'_>) -> HashSet<String> {
    let mut collector = IdentifierReferenceCollector::default();
    collector.visit_program(program);
    collector.identifiers
}

pub fn collect_runtime_imports_from_program<'a>(
    abs_path: &Path,
    program: &oxc_ast::ast::Program<'a>,
    referenced_identifiers: &HashSet<String>,
) -> Result<Vec<PathBuf>> {
    let mut imports = Vec::new();
    for stmt in &program.body {
        if let Statement::ImportDeclaration(import) = stmt {
            if !is_runtime_import(import) || !is_import_used(import, referenced_identifiers) {
                continue;
            }
            if let Some(resolved) = resolve_import(abs_path, import.source.value.as_str()) {
                imports.push(resolved);
            }
        }
    }
    Ok(imports)
}

pub fn is_import_used(
    import: &oxc_ast::ast::ImportDeclaration<'_>,
    referenced_identifiers: &HashSet<String>,
) -> bool {
    let Some(specifiers) = &import.specifiers else {
        return true;
    };
    if specifiers.is_empty() {
        return true;
    }

    for specifier in specifiers {
        let local_name = match specifier {
            ImportDeclarationSpecifier::ImportDefaultSpecifier(default_import) => {
                default_import.local.name.as_ref()
            }
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(namespace_import) => {
                namespace_import.local.name.as_ref()
            }
            ImportDeclarationSpecifier::ImportSpecifier(import_specifier) => {
                import_specifier.local.name.as_ref()
            }
        };
        if referenced_identifiers.contains(local_name) {
            return true;
        }
    }

    false
}

pub fn collect_imports_from_program<'a>(
    abs_path: &Path,
    program: &oxc_ast::ast::Program<'a>,
    import_cache: &mut HashMap<PathBuf, Vec<PathBuf>>,
) -> Result<Vec<PathBuf>> {
    if let Some(cached_imports) = import_cache.get(abs_path) {
        return Ok(cached_imports.clone());
    }

    let mut imports = Vec::new();
    for stmt in &program.body {
        match stmt {
            Statement::ImportDeclaration(import) if is_runtime_import(import) => {
                if let Some(resolved) = resolve_import(abs_path, import.source.value.as_str()) {
                    imports.push(resolved);
                }
            }
            Statement::ExportNamedDeclaration(export) => {
                if !is_runtime_export(export) {
                    continue;
                }
                if let Some(source) = &export.source {
                    if let Some(resolved) = resolve_import(abs_path, source.value.as_str()) {
                        imports.push(resolved);
                    }
                }
            }
            Statement::ExportAllDeclaration(export) => {
                if export.export_kind == ImportOrExportKind::Type {
                    continue;
                }
                if let Some(resolved) = resolve_import(abs_path, export.source.value.as_str()) {
                    imports.push(resolved);
                }
            }
            _ => {}
        }
    }

    import_cache.insert(abs_path.to_path_buf(), imports.clone());
    Ok(imports)
}
