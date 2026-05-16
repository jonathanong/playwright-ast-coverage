use crate::analyze::import_table::ImportTable;
use oxc_ast::ast::{JSXElementName, JSXMemberExpression, JSXMemberExpressionObject, Program};
use oxc_ast_visit::{walk, Visit};
use std::path::PathBuf;

struct JsxChildrenVisitor<'a> {
    import_table: &'a ImportTable,
    children: Vec<(PathBuf, String)>,
}

impl<'a> JsxChildrenVisitor<'a> {
    fn new(import_table: &'a ImportTable) -> Self {
        Self {
            import_table,
            children: Vec::new(),
        }
    }
}

impl<'a> Visit<'a> for JsxChildrenVisitor<'a> {
    fn visit_jsx_element(&mut self, elem: &oxc_ast::ast::JSXElement<'a>) {
        let local_name = match &elem.opening_element.name {
            JSXElementName::IdentifierReference(id) => {
                let n = id.name.as_ref();
                if n.chars().next().is_some_and(|c| c.is_uppercase()) {
                    Some(n.to_string())
                } else {
                    None
                }
            }
            JSXElementName::MemberExpression(m) => Some(jsx_member_root(m)),
            _ => None,
        };
        if let Some(local_name) = local_name {
            let key = local_name.split('.').next().unwrap_or(&local_name);
            if let Some(entry) = self.import_table.get(key) {
                let exported = if local_name.contains('.') {
                    local_name
                        .split_once('.')
                        .map(|(_, rest)| rest.to_string())
                        .unwrap_or_else(|| entry.exported_name.clone())
                } else {
                    entry.exported_name.clone()
                };
                self.children.push((entry.resolved_path.clone(), exported));
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

pub(crate) fn collect_jsx_children(
    program: &Program<'_>,
    import_table: &ImportTable,
) -> Vec<(PathBuf, String)> {
    let mut visitor = JsxChildrenVisitor::new(import_table);
    visitor.visit_program(program);
    visitor.children
}
