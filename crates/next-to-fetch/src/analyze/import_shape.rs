use oxc_ast::ast::{
    Declaration, ExportNamedDeclaration, ImportDeclarationSpecifier, ImportOrExportKind,
};

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

    // oxc sets export_kind=Type for TSTypeAliasDeclaration, TSInterfaceDeclaration,
    // and TSGlobalDeclaration, so those variants are caught above and never reach here.
    match &export.declaration {
        Some(Declaration::VariableDeclaration(d)) => return !d.declare,
        Some(Declaration::FunctionDeclaration(d)) => return !d.declare,
        Some(Declaration::ClassDeclaration(d)) => return !d.declare,
        Some(Declaration::TSEnumDeclaration(d)) => return !d.declare,
        Some(Declaration::TSModuleDeclaration(d)) => return !d.declare,
        Some(Declaration::TSImportEqualsDeclaration(d)) => {
            return d.import_kind == ImportOrExportKind::Value
        }
        Some(_) | None => {}
    }

    if export.specifiers.is_empty() {
        return true;
    }
    export
        .specifiers
        .iter()
        .any(|spec| spec.export_kind == ImportOrExportKind::Value)
}
