use super::scoped_defaults::{
    app_selector_value, collect_scoped_static_identifier_defaults, jsx_attribute_name,
    ScopedStaticIdentifierDefault,
};
use super::types::{AppSelector, SelectorRegexes};
use super::HTML_ID_ATTRIBUTE;
use crate::ast;
use oxc_ast_visit::Visit;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[cfg(test)]
use anyhow::Result;
#[cfg(test)]
use std::collections::BTreeSet;
#[cfg(test)]
use walkdir::WalkDir;

#[cfg(test)]
pub fn collect_app_selectors(
    frontend_root: &Path,
    attributes: &[String],
) -> Result<Vec<AppSelector>> {
    use super::is_skipped_dir;
    use super::is_source_file;
    let component_attributes = BTreeMap::new();
    if frontend_root.exists() {
        let mut selectors = BTreeSet::new();
        for entry in WalkDir::new(frontend_root)
            .into_iter()
            .filter_entry(|entry| !is_skipped_dir(entry.path()))
            .filter_map(|entry| entry.ok())
        {
            let path = entry.path();
            if !path.is_file() || !is_source_file(path) {
                continue;
            }
            let source = std::fs::read_to_string(path)?;
            selectors.extend(extract_app_selectors(
                path,
                &source,
                attributes,
                &component_attributes,
            )?);
        }
        Ok(selectors.into_iter().collect())
    } else {
        Ok(Vec::new())
    }
}

#[cfg(test)]
pub fn extract_app_selectors(
    path: &Path,
    source: &str,
    attributes: &[String],
    component_attributes: &BTreeMap<String, String>,
) -> Result<Vec<AppSelector>> {
    use super::regex_mod::compile_selector_regexes;
    let regexes = compile_selector_regexes(attributes, component_attributes);
    extract_app_selectors_with_regexes(path, source, &regexes)
}

pub fn extract_app_selectors_with_regexes(
    path: &Path,
    source: &str,
    regexes: &SelectorRegexes,
) -> anyhow::Result<Vec<AppSelector>> {
    ast::with_program(path, source, |program, source| {
        let scoped_static_identifier_defaults = collect_scoped_static_identifier_defaults(program);
        let mut visitor = AppSelectorVisitor {
            path,
            source,
            attributes: &regexes.app_attributes,
            component_attributes: &regexes.component_attributes,
            html_ids: regexes.html_ids,
            scoped_static_identifier_defaults: &scoped_static_identifier_defaults,
            selectors: Vec::new(),
        };
        visitor.visit_program(program);
        visitor.selectors
    })
}

struct AppSelectorVisitor<'a, 'r> {
    path: &'r Path,
    source: &'a str,
    attributes: &'r [String],
    component_attributes: &'r BTreeMap<String, String>,
    html_ids: bool,
    scoped_static_identifier_defaults: &'r [ScopedStaticIdentifierDefault],
    selectors: Vec<AppSelector>,
}

impl<'a> oxc_ast_visit::Visit<'a> for AppSelectorVisitor<'a, '_> {
    fn visit_jsx_opening_element(&mut self, element: &oxc_ast::ast::JSXOpeningElement<'a>) {
        let component = is_component_jsx_element_name(&element.name);
        for item in &element.attributes {
            let oxc_ast::ast::JSXAttributeItem::Attribute(attribute) = item else {
                continue;
            };
            let Some(name) = jsx_attribute_name(&attribute.name) else {
                continue;
            };
            let Some(mapped_attribute) = self.mapped_attribute(name, component) else {
                continue;
            };

            if let Some(value) = app_selector_value(
                attribute.value.as_ref(),
                self.source,
                self.scoped_static_identifier_defaults,
            ) {
                self.selectors.push(AppSelector {
                    file: PathBuf::from(self.path),
                    attribute: mapped_attribute.to_string(),
                    value,
                });
            }
        }

        oxc_ast_visit::walk::walk_jsx_opening_element(self, element);
    }
}

impl AppSelectorVisitor<'_, '_> {
    fn mapped_attribute<'a>(&'a self, name: &'a str, component: bool) -> Option<&'a str> {
        if self.attributes.iter().any(|attribute| attribute == name) {
            return Some(name);
        }
        if self.html_ids && !component && name == HTML_ID_ATTRIBUTE {
            return Some(HTML_ID_ATTRIBUTE);
        }
        if component {
            return self.component_attributes.get(name).map(String::as_str);
        }
        None
    }
}

pub(super) fn is_component_jsx_element_name(name: &oxc_ast::ast::JSXElementName<'_>) -> bool {
    match name {
        oxc_ast::ast::JSXElementName::Identifier(identifier) => identifier
            .name
            .chars()
            .next()
            .is_some_and(|ch| !ch.is_ascii_lowercase()),
        oxc_ast::ast::JSXElementName::IdentifierReference(identifier) => identifier
            .name
            .chars()
            .next()
            .is_some_and(|ch| !ch.is_ascii_lowercase()),
        oxc_ast::ast::JSXElementName::MemberExpression(_) => true,
        oxc_ast::ast::JSXElementName::NamespacedName(_)
        | oxc_ast::ast::JSXElementName::ThisExpression(_) => false,
    }
}
