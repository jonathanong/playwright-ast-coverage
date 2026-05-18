use super::super::{is_class_component, is_component_expr, is_component_name};
use oxc_ast::ast::{
    BindingPattern, Declaration, ExportNamedDeclaration, Program, Statement, VariableDeclaration,
};
use oxc_span::Span;
use std::collections::HashMap;

pub(in super::super) fn collect_local_vars<'a>(program: &'a Program<'a>) -> HashMap<&'a str, Span> {
    let mut local_vars = HashMap::new();
    for stmt in &program.body {
        match stmt {
            Statement::VariableDeclaration(v) => {
                collect_component_declarations(v, |name, span| {
                    local_vars.insert(name, span);
                });
            }
            Statement::FunctionDeclaration(f) => {
                if let Some(id) = &f.id {
                    let name = id.name.as_ref();
                    if is_component_name(name) {
                        local_vars.insert(name, f.span);
                    }
                }
            }
            Statement::ClassDeclaration(c) => {
                if let Some(id) = &c.id {
                    let name = id.name.as_ref();
                    if is_component_name(name) && is_class_component(c) {
                        local_vars.insert(name, c.span);
                    }
                }
            }
            Statement::ExportNamedDeclaration(export) => {
                collect_local_export_declaration(export, &mut local_vars);
            }
            _ => {}
        }
    }
    local_vars
}

fn collect_local_export_declaration<'a>(
    export: &'a ExportNamedDeclaration<'a>,
    local_vars: &mut HashMap<&'a str, Span>,
) {
    let Some(decl) = &export.declaration else {
        return;
    };
    match decl {
        Declaration::FunctionDeclaration(f) => {
            if let Some(id) = &f.id {
                let name = id.name.as_ref();
                if is_component_name(name) {
                    local_vars.insert(name, f.span);
                }
            }
        }
        Declaration::VariableDeclaration(v) => {
            collect_component_declarations(v, |name, span| {
                local_vars.insert(name, span);
            });
        }
        Declaration::ClassDeclaration(c) => {
            if let Some(id) = &c.id {
                let name = id.name.as_ref();
                if is_component_name(name) && is_class_component(c) {
                    local_vars.insert(name, c.span);
                }
            }
        }
        _ => {}
    }
}

pub(super) fn collect_component_declarations<'a, F>(decl: &'a VariableDeclaration<'a>, mut f: F)
where
    F: FnMut(&'a str, Span),
{
    for declarator in &decl.declarations {
        if let BindingPattern::BindingIdentifier(id) = &declarator.id {
            let name = id.name.as_ref();
            if is_component_name(name) {
                if let Some(init) = &declarator.init {
                    if is_component_expr(init) {
                        f(name, declarator.span);
                    }
                }
            }
        }
    }
}
