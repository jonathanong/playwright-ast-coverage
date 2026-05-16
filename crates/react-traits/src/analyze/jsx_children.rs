use crate::analyze::components::{is_class_component, is_component_name};
use crate::analyze::import_table::ImportTable;
use oxc_ast::ast::{
    BindingPattern, Declaration, ExportDefaultDeclarationKind, Expression, JSXElementName,
    JSXMemberExpression, JSXMemberExpressionObject, Program, Statement,
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
        let (root_name, member_suffix) = match &elem.opening_element.name {
            JSXElementName::IdentifierReference(id) => {
                let n = id.name.as_ref();
                let root = n
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_uppercase())
                    .then(|| n.to_string());
                (root, None)
            }
            JSXElementName::MemberExpression(m) => {
                let root = jsx_member_root(m);
                let suffix = jsx_member_suffix(m);
                (Some(root), Some(suffix))
            }
            _ => (None, None),
        };
        if let Some(root) = root_name {
            if let Some(entry) = self.import_table.get(root.as_str()) {
                // If there's a member suffix and the root is a namespace import ("*"),
                // use the suffix as the exported name; otherwise use the import's exported name.
                let exported = if let Some(ref suffix) = member_suffix {
                    if entry.exported_name == "*" {
                        suffix.clone()
                    } else {
                        entry.exported_name.clone()
                    }
                } else {
                    entry.exported_name.clone()
                };
                self.children.push((entry.resolved_path.clone(), exported));
            } else if let Some(exported_name) = self.local_components.get(root.as_str()) {
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

fn jsx_member_suffix(m: &JSXMemberExpression<'_>) -> String {
    m.property.name.to_string()
}

fn collect_local_components(program: &Program<'_>) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    for stmt in &program.body {
        match stmt {
            Statement::ExportDefaultDeclaration(e) => {
                match &e.declaration {
                    ExportDefaultDeclarationKind::Identifier(id) => {
                        // `export default Page` — map local symbol "Page" -> "default"
                        map.insert(id.name.as_ref().to_string(), "default".to_string());
                    }
                    ExportDefaultDeclarationKind::CallExpression(call) => {
                        // `export default memo(Page)` — map wrapped identifier -> "default"
                        if let Some(first_arg) = call.arguments.first() {
                            if let Some(Expression::Identifier(id)) = first_arg.as_expression() {
                                map.insert(id.name.as_ref().to_string(), "default".to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
            Statement::ExportNamedDeclaration(e) => {
                if let Some(decl) = &e.declaration {
                    // Inline export: local name == exported name
                    match decl {
                        Declaration::FunctionDeclaration(f) if f.id.is_some() => {
                            let id = f.id.as_ref().unwrap();
                            if is_component_name(id.name.as_ref()) {
                                let n = id.name.as_ref().to_string();
                                map.insert(n.clone(), n);
                            }
                        }
                        Declaration::VariableDeclaration(v) => {
                            for d in &v.declarations {
                                if let BindingPattern::BindingIdentifier(id) = &d.id {
                                    if is_component_name(id.name.as_ref()) {
                                        let n = id.name.as_ref().to_string();
                                        map.insert(n.clone(), n);
                                    }
                                }
                            }
                        }
                        Declaration::ClassDeclaration(c)
                            if c.id.is_some() && is_class_component(c) =>
                        {
                            let id = c.id.as_ref().unwrap();
                            if is_component_name(id.name.as_ref()) {
                                let n = id.name.as_ref().to_string();
                                map.insert(n.clone(), n);
                            }
                        }
                        _ => {}
                    }
                } else {
                    // `export { Foo, Bar as Baz }` — local symbol -> exported name
                    for spec in &e.specifiers {
                        let local = spec.local.name().as_ref().to_string();
                        let exported = spec.exported.name().as_ref().to_string();
                        if is_component_name(&local) {
                            map.insert(local, exported);
                        }
                    }
                }
            }
            _ => {}
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
