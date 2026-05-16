use crate::analyze::components::{extract_components, is_component_expr};
use crate::analyze::import_table::ImportTable;
use oxc_ast::ast::{
    BindingPattern, JSXElementName, JSXMemberExpression, JSXMemberExpressionObject, Program,
    Statement,
};
use oxc_ast_visit::{walk, Visit};
use oxc_span::Span;
use std::collections::HashMap;
use std::path::PathBuf;

struct JsxChildrenVisitor<'a> {
    import_table: &'a ImportTable,
    local_components: &'a HashMap<String, String>,
    file_path: &'a PathBuf,
    span: Span,
    children: Vec<(PathBuf, String)>,
}

impl<'a> JsxChildrenVisitor<'a> {
    fn new(
        import_table: &'a ImportTable,
        local_components: &'a HashMap<String, String>,
        file_path: &'a PathBuf,
        span: Span,
    ) -> Self {
        Self {
            import_table,
            local_components,
            file_path,
            span,
            children: Vec::new(),
        }
    }
}

impl<'a> Visit<'a> for JsxChildrenVisitor<'a> {
    fn visit_jsx_element(&mut self, elem: &oxc_ast::ast::JSXElement<'a>) {
        let s = elem.span;
        if s.start < self.span.start || s.end > self.span.end {
            walk::walk_jsx_element(self, elem);
            return;
        }
        let local_name = match &elem.opening_element.name {
            JSXElementName::IdentifierReference(id) => {
                let n = id.name.as_ref();
                n.chars()
                    .next()
                    .is_some_and(|c| c.is_uppercase())
                    .then(|| n.to_string())
            }
            JSXElementName::MemberExpression(m) => Some(jsx_member_root(m)),
            _ => None,
        };
        if let Some(local_name) = local_name {
            let key = local_name.split('.').next().unwrap_or(&local_name);
            if let Some(entry) = self.import_table.get(key) {
                let exported = entry.exported_name.clone();
                self.children.push((entry.resolved_path.clone(), exported));
            } else if let Some(exported_name) = self.local_components.get(key) {
                self.children
                    .push((self.file_path.clone(), exported_name.clone()));
            }
        }
        walk::walk_jsx_element(self, elem);
    }
}

fn jsx_member_root(m: &JSXMemberExpression<'_>) -> String {
    match &m.object {
        JSXMemberExpressionObject::IdentifierReference(id) => id.name.to_string(),
        JSXMemberExpressionObject::MemberExpression(m2) => jsx_member_root(m2),
        JSXMemberExpressionObject::ThisExpression(_) => String::new(),
    }
}

fn collect_local_components(program: &Program<'_>) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    // Named exports from extract_components
    for def in extract_components(program) {
        if def.name != "default" {
            map.insert(def.name.clone(), def.name);
        } else {
            // Try to find the function name for the default export from top-level vars
            for stmt in &program.body {
                if let Statement::VariableDeclaration(v) = stmt {
                    for decl in &v.declarations {
                        if let BindingPattern::BindingIdentifier(id) = &decl.id {
                            let name = id.name.as_ref();
                            if let Some(init) = &decl.init {
                                if is_component_expr(init) {
                                    map.insert(name.to_string(), "default".to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    map
}

#[cfg(test)]
mod tests;

pub(crate) fn collect_jsx_children(
    program: &Program<'_>,
    import_table: &ImportTable,
    file_path: &PathBuf,
    span: Span,
) -> Vec<(PathBuf, String)> {
    let local_components = collect_local_components(program);
    let mut visitor = JsxChildrenVisitor::new(import_table, &local_components, file_path, span);
    visitor.visit_program(program);
    visitor.children
}
