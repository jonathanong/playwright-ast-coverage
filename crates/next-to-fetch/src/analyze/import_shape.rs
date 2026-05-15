use oxc_ast::ast::{Declaration, ExportNamedDeclaration, ImportDeclarationSpecifier, ImportOrExportKind};

pub(crate) fn is_runtime_import(import: &oxc_ast::ast::ImportDeclaration) -> bool {
    if import.import_kind == ImportOrExportKind::Type {
        return false;
    }

    let Some(specifiers) = &import.specifiers else {
        return true;
    };
    if specifiers.is_empty() {
        return true;
    }

    for specifier in specifiers {
        match specifier {
            ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => return true,
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => return true,
            ImportDeclarationSpecifier::ImportSpecifier(import_specifier) => {
                if import_specifier.import_kind == ImportOrExportKind::Value {
                    return true;
                }
            }
        }
    }

    false
}

pub(crate) fn is_runtime_export(export: &ExportNamedDeclaration) -> bool {
    if export.export_kind == ImportOrExportKind::Type {
        return false;
    }

    if let Some(decl) = &export.declaration {
        return !matches!(
            decl,
            Declaration::TSTypeAliasDeclaration(_) | Declaration::TSInterfaceDeclaration(_)
        );
    }

    if export.specifiers.is_empty() {
        return true;
    }
    export
        .specifiers
        .iter()
        .any(|spec| spec.export_kind == ImportOrExportKind::Value)
}
