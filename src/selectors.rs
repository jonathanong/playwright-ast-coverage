use crate::{ast, playwright_tests};
#[cfg(test)]
use anyhow::Result;
use oxc_ast_visit::Visit;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
#[cfg(test)]
use walkdir::WalkDir;

const SOURCE_EXTS: &[&str] = &["ts", "tsx", "js", "jsx", "mts", "cts", "mjs", "cjs"];

pub struct SelectorRegexes {
    app_attributes: Vec<String>,
    component_attributes: BTreeMap<String, String>,
    playwright_attributes: Vec<AttributeRegex>,
}

struct AttributeRegex {
    attribute: String,
    regex: Regex,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct AppSelector {
    pub file: PathBuf,
    pub attribute: String,
    pub value: AppSelectorValue,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum AppSelectorValue {
    Exact(String),
    Template(TemplatePattern),
    Unsupported(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TemplatePattern {
    raw: String,
    parts: Vec<String>,
    starts_static: bool,
    ends_static: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PlaywrightSelector {
    pub attribute: String,
    pub selector: String,
    matcher: SelectorMatcher,
}

#[derive(Clone, Debug)]
enum SelectorMatcher {
    Exact(String),
    Prefix(String),
    Suffix(String),
    Contains(String),
    Regex {
        pattern: String,
        compiled: Option<Regex>,
    },
}

pub fn compile_selector_regexes(
    attributes: &[String],
    component_attributes: &BTreeMap<String, String>,
) -> SelectorRegexes {
    let mut playwright_attributes: Vec<_> = attributes
        .iter()
        .chain(component_attributes.values())
        .cloned()
        .collect();
    playwright_attributes.sort();
    playwright_attributes.dedup();

    SelectorRegexes {
        app_attributes: attributes.to_vec(),
        component_attributes: component_attributes.clone(),
        playwright_attributes: playwright_attributes
            .iter()
            .map(|attribute| AttributeRegex {
                attribute: attribute.clone(),
                regex: playwright_selector_regex(attribute),
            })
            .collect(),
    }
}

impl AppSelector {
    pub fn display_value(&self) -> String {
        self.value.display_value()
    }

    pub fn unsupported_dynamic(&self) -> bool {
        matches!(self.value, AppSelectorValue::Unsupported(_))
    }

    pub fn matches_playwright(&self, selector: &PlaywrightSelector) -> bool {
        self.attribute == selector.attribute && self.value.matches_selector(&selector.matcher)
    }
}

impl PlaywrightSelector {
    pub fn exact_value(&self) -> Option<&str> {
        match &self.matcher {
            SelectorMatcher::Exact(value) => Some(value),
            _ => None,
        }
    }
}

impl PartialEq for SelectorMatcher {
    fn eq(&self, other: &Self) -> bool {
        self.cmp_key() == other.cmp_key()
    }
}

impl Eq for SelectorMatcher {}

impl PartialOrd for SelectorMatcher {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SelectorMatcher {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cmp_key().cmp(&other.cmp_key())
    }
}

impl AppSelectorValue {
    fn display_value(&self) -> String {
        match self {
            Self::Exact(value) => value.clone(),
            Self::Template(pattern) => pattern.raw.clone(),
            Self::Unsupported(value) => format!("{{{value}}}"),
        }
    }

    fn matches_selector(&self, matcher: &SelectorMatcher) -> bool {
        match self {
            Self::Exact(value) => matcher.matches_value(value),
            Self::Template(pattern) => matcher.matches_pattern(pattern),
            Self::Unsupported(_) => false,
        }
    }
}

impl TemplatePattern {
    fn new(raw: &str) -> Option<Self> {
        let parts = template_parts(raw);
        if parts.iter().all(|part| part.is_empty()) {
            return None;
        }
        Some(Self {
            raw: raw.to_string(),
            parts,
            starts_static: !raw.starts_with("${"),
            ends_static: !raw.ends_with('}'),
        })
    }

    fn matches_exact(&self, value: &str) -> bool {
        let non_empty: Vec<&str> = self
            .parts
            .iter()
            .filter(|part| !part.is_empty())
            .map(String::as_str)
            .collect();
        if non_empty.is_empty() {
            return false;
        }
        if self.starts_static && !value.starts_with(non_empty[0]) {
            return false;
        }
        if self.ends_static && !value.ends_with(non_empty[non_empty.len() - 1]) {
            return false;
        }

        let mut offset = 0;
        for part in non_empty {
            let Some(index) = value[offset..].find(part) else {
                return false;
            };
            offset += index + part.len();
        }
        true
    }

    fn sample(&self) -> String {
        let mut sample = String::new();
        for (index, part) in self.parts.iter().enumerate() {
            if index > 0 {
                sample.push('x');
            }
            sample.push_str(part);
        }
        sample
    }

    fn first_static(&self) -> Option<&str> {
        self.parts
            .iter()
            .find(|part| !part.is_empty())
            .map(String::as_str)
    }

    fn last_static(&self) -> Option<&str> {
        self.parts
            .iter()
            .rev()
            .find(|part| !part.is_empty())
            .map(String::as_str)
    }

    fn contains_static(&self, needle: &str) -> bool {
        self.parts
            .iter()
            .any(|part| !part.is_empty() && (part.contains(needle) || needle.contains(part)))
    }
}

impl SelectorMatcher {
    fn cmp_key(&self) -> (u8, &str) {
        match self {
            Self::Exact(value) => (0, value),
            Self::Prefix(value) => (1, value),
            Self::Suffix(value) => (2, value),
            Self::Contains(value) => (3, value),
            Self::Regex { pattern, .. } => (4, pattern),
        }
    }

    fn matches_value(&self, value: &str) -> bool {
        match self {
            Self::Exact(expected) => value == expected,
            Self::Prefix(prefix) => value.starts_with(prefix),
            Self::Suffix(suffix) => value.ends_with(suffix),
            Self::Contains(part) => value.contains(part),
            Self::Regex { compiled, .. } => {
                compiled.as_ref().is_some_and(|regex| regex.is_match(value))
            }
        }
    }

    fn matches_pattern(&self, pattern: &TemplatePattern) -> bool {
        match self {
            Self::Exact(value) => pattern.matches_exact(value),
            Self::Prefix(prefix) => pattern
                .first_static()
                .is_some_and(|part| part.starts_with(prefix) || prefix.starts_with(part)),
            Self::Suffix(suffix) => pattern
                .last_static()
                .is_some_and(|part| part.ends_with(suffix) || suffix.ends_with(part)),
            Self::Contains(part) => pattern.contains_static(part),
            Self::Regex { compiled, .. } => compiled
                .as_ref()
                .is_some_and(|regex| regex.is_match(&pattern.sample())),
        }
    }
}

#[cfg(test)]
pub fn collect_app_selectors(
    frontend_root: &Path,
    attributes: &[String],
) -> Result<Vec<AppSelector>> {
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
            scoped_static_identifier_defaults: &scoped_static_identifier_defaults,
            selectors: BTreeSet::new(),
        };
        visitor.visit_program(program);
        visitor.selectors.into_iter().collect()
    })
}

#[cfg(test)]
pub fn extract_playwright_selectors(
    source: &str,
    selector_attributes: &[String],
    test_id_attributes: &[String],
) -> Vec<PlaywrightSelector> {
    let component_attributes = BTreeMap::new();
    let regexes = compile_selector_regexes(selector_attributes, &component_attributes);
    extract_playwright_selectors_with_regexes(
        Path::new("fixture.ts"),
        source,
        &regexes,
        test_id_attributes,
    )
    .expect("fixture should parse")
}

#[cfg(test)]
pub fn extract_playwright_selectors_with_regexes(
    path: &Path,
    source: &str,
    regexes: &SelectorRegexes,
    test_id_attributes: &[String],
) -> anyhow::Result<Vec<PlaywrightSelector>> {
    ast::with_program(path, source, |program, source| {
        extract_playwright_selectors_from_program(program, source, regexes, test_id_attributes)
    })
}

#[cfg(test)]
pub fn extract_playwright_selector_occurrences(
    source: &str,
    selector_attributes: &[String],
    test_id_attributes: &[String],
) -> Vec<(String, playwright_tests::TestStatus)> {
    let component_attributes = BTreeMap::new();
    let regexes = compile_selector_regexes(selector_attributes, &component_attributes);
    ast::with_program(Path::new("fixture.ts"), source, |program, source| {
        extract_playwright_selector_occurrences_from_program(
            program,
            source,
            &regexes,
            test_id_attributes,
        )
        .into_iter()
        .map(|occurrence| (occurrence.value.selector, occurrence.status))
        .collect()
    })
    .expect("fixture should parse")
}

#[cfg(test)]
pub fn extract_playwright_selectors_from_program(
    program: &oxc_ast::ast::Program<'_>,
    source: &str,
    regexes: &SelectorRegexes,
    test_id_attributes: &[String],
) -> Vec<PlaywrightSelector> {
    extract_playwright_selector_occurrences_from_program(
        program,
        source,
        regexes,
        test_id_attributes,
    )
    .into_iter()
    .map(|occurrence| occurrence.value)
    .collect()
}

pub fn extract_playwright_selector_occurrences_from_program(
    program: &oxc_ast::ast::Program<'_>,
    source: &str,
    regexes: &SelectorRegexes,
    test_id_attributes: &[String],
) -> Vec<playwright_tests::TestOccurrence<PlaywrightSelector>> {
    let mut visitor = PlaywrightSelectorVisitor {
        source,
        regexes,
        test_id_attributes,
        status: playwright_tests::TestStatus::Active,
        annotation_status: playwright_tests::TestStatus::Active,
        selectors: Vec::new(),
    };
    visitor.visit_program(program);
    visitor.selectors.sort();
    visitor.selectors.dedup();
    visitor.selectors
}

struct AppSelectorVisitor<'a, 'r> {
    path: &'r Path,
    source: &'a str,
    attributes: &'r [String],
    component_attributes: &'r BTreeMap<String, String>,
    scoped_static_identifier_defaults: &'r [ScopedStaticIdentifierDefault],
    selectors: BTreeSet<AppSelector>,
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
                self.selectors.insert(AppSelector {
                    file: self.path.to_path_buf(),
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
        if component {
            return self.component_attributes.get(name).map(String::as_str);
        }
        None
    }
}

fn is_component_jsx_element_name(name: &oxc_ast::ast::JSXElementName<'_>) -> bool {
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

struct ScopedStaticIdentifierDefault {
    name: String,
    value: String,
    scope: Span,
}

struct ScopedDefaultVisitor {
    defaults: Vec<ScopedStaticIdentifierDefault>,
}

impl<'a> oxc_ast_visit::Visit<'a> for ScopedDefaultVisitor {
    fn visit_function(&mut self, function: &oxc_ast::ast::Function<'a>, flags: ScopeFlags) {
        if let Some(body) = &function.body {
            self.collect_function_defaults(&function.params, body.span());
        }
        oxc_ast_visit::walk::walk_function(self, function, flags);
    }

    fn visit_arrow_function_expression(
        &mut self,
        arrow: &oxc_ast::ast::ArrowFunctionExpression<'a>,
    ) {
        self.collect_function_defaults(&arrow.params, arrow.body.span());
        oxc_ast_visit::walk::walk_arrow_function_expression(self, arrow);
    }
}

impl ScopedDefaultVisitor {
    fn collect_function_defaults(
        &mut self,
        params: &oxc_ast::ast::FormalParameters<'_>,
        scope: Span,
    ) {
        for param in &params.items {
            collect_static_defaults_from_binding(
                &param.pattern,
                param.initializer.as_deref(),
                scope,
                &mut self.defaults,
            );
        }
    }
}

struct PlaywrightSelectorVisitor<'a, 'r> {
    source: &'a str,
    regexes: &'r SelectorRegexes,
    test_id_attributes: &'r [String],
    status: playwright_tests::TestStatus,
    annotation_status: playwright_tests::TestStatus,
    selectors: Vec<playwright_tests::TestOccurrence<PlaywrightSelector>>,
}

impl<'a> oxc_ast_visit::Visit<'a> for PlaywrightSelectorVisitor<'a, '_> {
    fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
        if callee_is_static_member_named(&call.callee, "getByTestId") {
            extract_get_by_test_id_call(
                call,
                self.source,
                self.test_id_attributes,
                &mut |selector| self.insert(selector),
            );
        } else if let Some(argument_mode) = selector_argument_mode(&call.callee) {
            for selector in selector_argument_literals(call, self.source, argument_mode) {
                extract_css_attribute_selectors(
                    &selector,
                    &self.regexes.playwright_attributes,
                    &mut |selector| self.insert(selector),
                );
            }
        }

        let traversal = playwright_tests::test_callback_traversal(call, self.annotation_status);
        if traversal.is_none() {
            let callback_index = playwright_tests::callback_argument_index(call);
            if playwright_tests::annotation_status_for_call(call).is_some() {
                self.apply_annotation_call(call);
                for (index, argument) in call.arguments.iter().enumerate() {
                    if Some(index) != callback_index {
                        self.visit_argument(argument);
                    }
                }
                return;
            }
            oxc_ast_visit::walk::walk_call_expression(self, call);
            return;
        }

        let (callback_index, callback_status) = traversal.expect("checked traversal");
        for (index, argument) in call.arguments.iter().enumerate() {
            if index == callback_index {
                self.with_status(callback_status, |visitor| {
                    visitor.with_annotation_scope(|visitor| visitor.visit_argument(argument));
                });
            } else {
                self.visit_argument(argument);
            }
        }
    }

    fn visit_if_statement(&mut self, statement: &oxc_ast::ast::IfStatement<'a>) {
        self.visit_expression(&statement.test);
        let status = playwright_tests::status_for_if_branch(self.status);
        self.with_status(status, |visitor| {
            visitor.visit_statement(&statement.consequent);
            if let Some(alternate) = &statement.alternate {
                visitor.visit_statement(alternate);
            }
        });
    }

    fn visit_conditional_expression(
        &mut self,
        expression: &oxc_ast::ast::ConditionalExpression<'a>,
    ) {
        self.visit_expression(&expression.test);
        let status = playwright_tests::status_for_if_branch(self.status);
        self.with_status(status, |visitor| {
            visitor.visit_expression(&expression.consequent);
            visitor.visit_expression(&expression.alternate);
        });
    }

    fn visit_logical_expression(&mut self, expression: &oxc_ast::ast::LogicalExpression<'a>) {
        self.visit_expression(&expression.left);
        let status = playwright_tests::status_for_if_branch(self.status);
        self.with_status(status, |visitor| {
            visitor.visit_expression(&expression.right)
        });
    }
}

impl PlaywrightSelectorVisitor<'_, '_> {
    fn insert(&mut self, value: PlaywrightSelector) {
        self.selectors.push(playwright_tests::TestOccurrence {
            value,
            status: self.status.merge(self.annotation_status),
        });
    }

    fn with_status(&mut self, status: playwright_tests::TestStatus, visit: impl FnOnce(&mut Self)) {
        let previous = self.status;
        self.status = previous.merge(status);
        visit(self);
        self.status = previous;
    }

    fn with_annotation_scope(&mut self, visit: impl FnOnce(&mut Self)) {
        let previous = self.annotation_status;
        self.annotation_status = playwright_tests::TestStatus::Active;
        visit(self);
        self.annotation_status = previous;
    }

    fn apply_annotation_call(&mut self, call: &oxc_ast::ast::CallExpression<'_>) {
        if let Some(status) = playwright_tests::annotation_status_for_call(call) {
            let status = playwright_tests::merge_annotation_status(self.status, status);
            self.annotation_status =
                playwright_tests::merge_annotation_status(self.annotation_status, status);
        }
    }
}

fn jsx_attribute_name<'a>(name: &'a oxc_ast::ast::JSXAttributeName<'a>) -> Option<&'a str> {
    match name {
        oxc_ast::ast::JSXAttributeName::Identifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

fn app_selector_value(
    value: Option<&oxc_ast::ast::JSXAttributeValue<'_>>,
    source: &str,
    scoped_static_identifier_defaults: &[ScopedStaticIdentifierDefault],
) -> Option<AppSelectorValue> {
    match value? {
        oxc_ast::ast::JSXAttributeValue::StringLiteral(literal) => {
            Some(AppSelectorValue::Exact(literal.value.to_string()))
        }
        oxc_ast::ast::JSXAttributeValue::ExpressionContainer(container) => jsx_expression_value(
            &container.expression,
            source,
            scoped_static_identifier_defaults,
        ),
        _ => None,
    }
}

fn jsx_expression_value(
    expression: &oxc_ast::ast::JSXExpression<'_>,
    source: &str,
    scoped_static_identifier_defaults: &[ScopedStaticIdentifierDefault],
) -> Option<AppSelectorValue> {
    match expression {
        oxc_ast::ast::JSXExpression::StringLiteral(literal) => {
            Some(AppSelectorValue::Exact(literal.value.to_string()))
        }
        oxc_ast::ast::JSXExpression::TemplateLiteral(template) => {
            let raw = ast::template_literal_text(template, source);
            Some(
                TemplatePattern::new(&raw)
                    .map(AppSelectorValue::Template)
                    .unwrap_or_else(|| AppSelectorValue::Unsupported(raw)),
            )
        }
        oxc_ast::ast::JSXExpression::Identifier(identifier) => Some(
            scoped_static_default_for_identifier(
                identifier.name.as_str(),
                identifier.span(),
                scoped_static_identifier_defaults,
                source,
            )
            .map(AppSelectorValue::Exact)
            .unwrap_or_else(|| AppSelectorValue::Unsupported(identifier.name.to_string())),
        ),
        _ => Some(AppSelectorValue::Unsupported(
            ast::span_text(source, expression.span()).trim().to_string(),
        )),
    }
}

fn collect_scoped_static_identifier_defaults(
    program: &oxc_ast::ast::Program<'_>,
) -> Vec<ScopedStaticIdentifierDefault> {
    let mut visitor = ScopedDefaultVisitor {
        defaults: Vec::new(),
    };
    visitor.visit_program(program);
    visitor.defaults
}

fn collect_static_defaults_from_binding(
    pattern: &oxc_ast::ast::BindingPattern<'_>,
    initializer: Option<&oxc_ast::ast::Expression<'_>>,
    scope: Span,
    defaults: &mut Vec<ScopedStaticIdentifierDefault>,
) {
    if let (Some(name), Some(value)) = (
        binding_identifier_name(pattern),
        initializer_string(initializer),
    ) {
        defaults.push(ScopedStaticIdentifierDefault { name, value, scope });
    }

    match pattern {
        oxc_ast::ast::BindingPattern::AssignmentPattern(assignment) => {
            if let (Some(name), Some(value)) = (
                binding_identifier_name(&assignment.left),
                expression_string(&assignment.right),
            ) {
                defaults.push(ScopedStaticIdentifierDefault { name, value, scope });
            }
            collect_static_defaults_from_binding(&assignment.left, None, scope, defaults);
        }
        oxc_ast::ast::BindingPattern::ObjectPattern(object) => {
            for property in &object.properties {
                collect_static_defaults_from_binding(&property.value, None, scope, defaults);
            }
        }
        oxc_ast::ast::BindingPattern::ArrayPattern(array) => {
            for element in array.elements.iter().flatten() {
                collect_static_defaults_from_binding(element, None, scope, defaults);
            }
        }
        oxc_ast::ast::BindingPattern::BindingIdentifier(_) => {}
    }
}

fn binding_identifier_name(pattern: &oxc_ast::ast::BindingPattern<'_>) -> Option<String> {
    match pattern {
        oxc_ast::ast::BindingPattern::BindingIdentifier(identifier) => {
            Some(identifier.name.to_string())
        }
        _ => None,
    }
}

fn initializer_string(initializer: Option<&oxc_ast::ast::Expression<'_>>) -> Option<String> {
    initializer.and_then(expression_string)
}

fn expression_string(expression: &oxc_ast::ast::Expression<'_>) -> Option<String> {
    match expression {
        oxc_ast::ast::Expression::StringLiteral(literal) => Some(literal.value.to_string()),
        _ => None,
    }
}

fn scoped_static_default_for_identifier(
    name: &str,
    span: Span,
    defaults: &[ScopedStaticIdentifierDefault],
    source: &str,
) -> Option<String> {
    defaults
        .iter()
        .filter(|default| {
            default.name == name
                && default.scope.start <= span.start
                && span.end <= default.scope.end
        })
        .filter(|default| {
            !identifier_may_be_shadowed_or_reassigned(name, span, default.scope, source)
        })
        .min_by_key(|default| default.scope.end - default.scope.start)
        .map(|default| default.value.clone())
}

fn identifier_may_be_shadowed_or_reassigned(
    name: &str,
    span: Span,
    scope: Span,
    source: &str,
) -> bool {
    let start = scope.start as usize;
    let end = span.start as usize;
    let prefix = source.get(start..end).unwrap_or("");
    let prefix = code_only_text(prefix);
    let escaped = regex::escape(name);
    let declaration = Regex::new(&format!(r"\b(?:const|let|var)\s+{escaped}\b"))
        .expect("identifier declaration regex should compile");
    let destructuring_declaration = Regex::new(&format!(
        r"\b(?:const|let|var)\s+(?:\{{[^;]*\b{escaped}\b[^;]*\}}|\[[^;]*\b{escaped}\b[^;]*\])"
    ))
    .expect("identifier destructuring declaration regex should compile");
    let destructuring_parameter = Regex::new(&format!(
        r"\bfunction\b[^(]*\([^)]*(?:\{{[^)]*\b{escaped}\b[^)]*\}}|\[[^)]*\b{escaped}\b[^)]*\])"
    ))
    .expect("identifier destructuring parameter regex should compile");
    has_identifier_reassignment(&prefix, name)
        || declaration.is_match(&prefix)
        || destructuring_declaration.is_match(&prefix)
        || has_enclosing_shadow_binding(&prefix, &destructuring_parameter)
}

fn has_identifier_reassignment(source: &str, name: &str) -> bool {
    let source = code_only_text(source);
    for (index, _) in source.match_indices(name) {
        let before = source[..index].chars().next_back();
        let after_index = index + name.len();
        let after = source[after_index..].chars().next();
        if before.is_some_and(is_identifier_continue) || after.is_some_and(is_identifier_continue) {
            continue;
        }
        let before = source[..index].trim_end();
        if before.ends_with("++") || before.ends_with("--") {
            return true;
        }
        let rest = source[after_index..].trim_start();
        if rest.starts_with("++") || rest.starts_with("--") {
            return true;
        }
        if [
            "+=", "-=", "*=", "/=", "%=", "**=", "&&=", "||=", "??=", "<<=", ">>=", ">>>=",
        ]
        .iter()
        .any(|operator| rest.starts_with(operator))
        {
            return true;
        }
        if let Some(after_equals) = rest.strip_prefix('=') {
            if !after_equals.starts_with('=') && !after_equals.starts_with('>') {
                return true;
            }
        }
    }
    false
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphanumeric()
}

fn has_enclosing_shadow_binding(prefix: &str, binding: &Regex) -> bool {
    binding.find_iter(prefix).any(|matched| {
        let rest = &prefix[matched.end()..];
        let Some(block_start) = rest.find('{') else {
            return false;
        };
        if rest[..block_start].contains(';') {
            return false;
        }
        let mut depth = 0usize;
        for ch in rest[block_start..].chars() {
            match ch {
                '{' => depth += 1,
                '}' if depth <= 1 => return false,
                '}' => depth -= 1,
                _ => {}
            }
        }
        depth > 0
    })
}

fn code_only_text(source: &str) -> String {
    let mut chars = source.chars().peekable();
    let mut output = String::with_capacity(source.len());
    let mut in_single = false;
    let mut in_double = false;
    let mut in_template = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut escaped = false;
    let mut template_expression_depth = 0usize;

    while let Some(ch) = chars.next() {
        if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
                output.push(ch);
            } else {
                output.push(' ');
            }
            continue;
        }
        if in_block_comment {
            if ch == '*' && chars.peek().is_some_and(|next| *next == '/') {
                output.push(' ');
                output.push(' ');
                chars.next();
                in_block_comment = false;
            } else {
                output.push(if ch == '\n' { '\n' } else { ' ' });
            }
            continue;
        }
        if escaped {
            escaped = false;
            output.push(' ');
            continue;
        }
        if (in_single || in_double || in_template) && ch == '\\' {
            escaped = true;
            output.push(' ');
            continue;
        }
        if template_expression_depth > 0 {
            if ch == '{' {
                template_expression_depth += 1;
            } else if ch == '}' {
                template_expression_depth -= 1;
                if template_expression_depth == 0 {
                    in_template = true;
                    output.push(' ');
                    continue;
                }
            }
            output.push(ch);
            continue;
        }
        if in_single {
            in_single = ch != '\'';
            output.push(' ');
            continue;
        }
        if in_double {
            in_double = ch != '"';
            output.push(' ');
            continue;
        }
        if in_template {
            if ch == '$' && chars.peek().is_some_and(|next| *next == '{') {
                output.push(' ');
                output.push(' ');
                chars.next();
                in_template = false;
                template_expression_depth = 1;
            } else {
                in_template = ch != '`';
                output.push(' ');
            }
            continue;
        }

        if ch == '/' && chars.peek().is_some_and(|next| *next == '/') {
            output.push(' ');
            output.push(' ');
            chars.next();
            in_line_comment = true;
        } else if ch == '/' && chars.peek().is_some_and(|next| *next == '*') {
            output.push(' ');
            output.push(' ');
            chars.next();
            in_block_comment = true;
        } else if ch == '\'' {
            output.push(' ');
            in_single = true;
        } else if ch == '"' {
            output.push(' ');
            in_double = true;
        } else if ch == '`' {
            output.push(' ');
            in_template = true;
        } else {
            output.push(ch);
        }
    }

    output
}

fn extract_css_attribute_selectors(
    source: &str,
    attributes: &[AttributeRegex],
    insert: &mut impl FnMut(PlaywrightSelector),
) {
    for attribute in attributes {
        for captures in attribute.regex.captures_iter(source) {
            let op = captures.get(1).expect("operator capture").as_str();
            let value = first_capture(&captures, &[2, 3]).expect("value capture");
            insert(PlaywrightSelector {
                attribute: attribute.attribute.clone(),
                selector: captures
                    .get(0)
                    .expect("selector capture")
                    .as_str()
                    .to_string(),
                matcher: matcher_for_operator(op, value),
            });
        }
    }
}

fn callee_is_static_member_named(callee: &oxc_ast::ast::Expression<'_>, method: &str) -> bool {
    callee_static_member_name(callee).is_some_and(|name| name == method)
}

fn callee_static_member_name<'a>(callee: &'a oxc_ast::ast::Expression<'a>) -> Option<&'a str> {
    match callee {
        oxc_ast::ast::Expression::StaticMemberExpression(member) => {
            Some(member.property.name.as_str())
        }
        oxc_ast::ast::Expression::ParenthesizedExpression(parenthesized) => {
            callee_static_member_name(&parenthesized.expression)
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum SelectorArgumentMode {
    First,
    All,
}

fn selector_argument_mode(callee: &oxc_ast::ast::Expression<'_>) -> Option<SelectorArgumentMode> {
    match callee_static_member_name(callee)? {
        "dragAndDrop" => Some(SelectorArgumentMode::All),
        "$" | "$$" | "$$eval" | "$eval" | "check" | "click" | "dblclick" | "dispatchEvent"
        | "dragTo" | "evalOnSelector" | "evalOnSelectorAll" | "fill" | "focus" | "frameLocator"
        | "getAttribute" | "hover" | "innerHTML" | "innerText" | "inputValue" | "isChecked"
        | "isDisabled" | "isEditable" | "isEnabled" | "isHidden" | "isVisible" | "locator"
        | "press" | "selectOption" | "setChecked" | "tap" | "textContent" | "type" | "uncheck"
        | "waitForSelector" => Some(SelectorArgumentMode::First),
        _ => None,
    }
}

fn selector_argument_literals(
    call: &oxc_ast::ast::CallExpression<'_>,
    source: &str,
    mode: SelectorArgumentMode,
) -> Vec<String> {
    call.arguments
        .iter()
        .enumerate()
        .filter(|(index, _)| matches!(mode, SelectorArgumentMode::All) || *index == 0)
        .filter_map(|(_, argument)| match argument {
            oxc_ast::ast::Argument::StringLiteral(literal) => Some(literal.value.to_string()),
            oxc_ast::ast::Argument::TemplateLiteral(template) => {
                Some(ast::template_literal_text(template.as_ref(), source))
            }
            _ => None,
        })
        .collect()
}

fn extract_get_by_test_id_call(
    call: &oxc_ast::ast::CallExpression<'_>,
    source: &str,
    attributes: &[String],
    insert: &mut impl FnMut(PlaywrightSelector),
) {
    let Some(argument) = call.arguments.first() else {
        return;
    };

    let matcher = match argument {
        oxc_ast::ast::Argument::StringLiteral(literal) => Some((
            literal.value.to_string(),
            SelectorMatcher::Exact(literal.value.to_string()),
        )),
        oxc_ast::ast::Argument::TemplateLiteral(template) => {
            let value = ast::template_literal_text(template, source);
            Some((value.clone(), SelectorMatcher::Exact(value)))
        }
        oxc_ast::ast::Argument::RegExpLiteral(regex) => {
            let value = regex.regex.pattern.text.to_string();
            let compiled = Regex::new(&value).ok();
            Some((
                format!("/{value}/"),
                SelectorMatcher::Regex {
                    pattern: value,
                    compiled,
                },
            ))
        }
        _ => None,
    };

    let Some((display, matcher)) = matcher else {
        return;
    };
    for attribute in attributes {
        insert(PlaywrightSelector {
            attribute: attribute.clone(),
            selector: format!("getByTestId({display})"),
            matcher: matcher.clone(),
        });
    }
}

fn playwright_selector_regex(attribute: &str) -> Regex {
    let pattern = format!(
        r#"\[\s*{}\s*(=|\^=|\$=|\*=)\s*(?:"([^"]+)"|'([^']+)')\s*\]"#,
        regex::escape(attribute)
    );
    Regex::new(&pattern).expect("valid Playwright selector regex")
}

fn matcher_for_operator(operator: &str, value: &str) -> SelectorMatcher {
    match operator {
        "^=" => SelectorMatcher::Prefix(value.to_string()),
        "$=" => SelectorMatcher::Suffix(value.to_string()),
        "*=" => SelectorMatcher::Contains(value.to_string()),
        _ => SelectorMatcher::Exact(value.to_string()),
    }
}

fn first_capture<'a>(captures: &'a regex::Captures<'_>, indexes: &[usize]) -> Option<&'a str> {
    indexes
        .iter()
        .find_map(|index| captures.get(*index).map(|capture| capture.as_str()))
}

fn template_parts(source: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find("${") {
        parts.push(rest[..start].to_string());
        let expression = &rest[start + 2..];
        let Some(end) = expression.find('}') else {
            return vec![source.to_string()];
        };
        rest = &expression[end + 1..];
    }
    parts.push(rest.to_string());
    parts
}

pub fn is_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| SOURCE_EXTS.contains(&extension))
}

#[cfg(test)]
fn is_skipped_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, ".git" | "node_modules" | "target" | "dist" | "build"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playwright_tests::TestStatus;
    use crate::test_support::{fixture_path, fixture_source};

    fn attrs() -> Vec<String> {
        vec!["data-testid".to_string(), "data-pw".to_string()]
    }

    fn component_attrs() -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    #[test]
    fn extracts_static_jsx_selectors() {
        let source = fixture_source(&["selectors", "static-jsx.tsx"]);
        let selectors = extract_app_selectors(
            Path::new("app/page.tsx"),
            &source,
            &attrs(),
            &component_attrs(),
        )
        .unwrap();
        let mut values: Vec<String> = selectors.iter().map(AppSelector::display_value).collect();
        values.sort();
        assert_eq!(values, vec!["delete", "publish", "save"]);
    }

    #[test]
    fn extracts_template_and_unsupported_jsx_selectors() {
        let source = fixture_source(&["selectors", "template-and-unsupported.tsx"]);
        let selectors = extract_app_selectors(
            Path::new("app/page.tsx"),
            &source,
            &attrs(),
            &component_attrs(),
        )
        .unwrap();
        assert!(selectors
            .iter()
            .any(|selector| selector.display_value() == "user-${id}"));
        assert!(selectors.iter().any(AppSelector::unsupported_dynamic));
    }

    #[test]
    fn maps_component_selector_attributes_to_dom_attributes() {
        let mut component_attributes = BTreeMap::new();
        component_attributes.insert("dataPw".to_string(), "data-pw".to_string());
        let selectors = extract_app_selectors(
            Path::new("app/page.tsx"),
            r#"
            export function Page() {
                return <>
                    <SaveButton dataPw="save" />
                    <_SaveButton dataPw="private" />
                    <$SaveButton dataPw="dollar" />
                    <UI.Button dataPw="publish" />
                    <button dataPw="ignored" />
                    <custom-element dataPw="ignored-custom" />
                    <SaveButton data-pw="legacy" />
                    <SaveButton {...props} />
                </>;
            }
            "#,
            &attrs(),
            &component_attributes,
        )
        .unwrap();

        let values: BTreeSet<(String, String)> = selectors
            .iter()
            .map(|selector| (selector.attribute.clone(), selector.display_value()))
            .collect();
        assert_eq!(
            values,
            BTreeSet::from([
                ("data-pw".to_string(), "legacy".to_string()),
                ("data-pw".to_string(), "dollar".to_string()),
                ("data-pw".to_string(), "publish".to_string()),
                ("data-pw".to_string(), "private".to_string()),
                ("data-pw".to_string(), "save".to_string()),
            ])
        );
    }

    #[test]
    fn extracts_static_identifier_default_jsx_selectors() {
        let selectors = extract_app_selectors(
            Path::new("app/page.tsx"),
            r#"
            export function Link({ 'data-pw': dataPw = 'rss-feed-link' }) {
                return <a data-pw={dataPw}>RSS</a>;
            }

            export function Button({ passThrough }) {
                return (
                    <>
                        <button data-pw={passThrough}>Save</button>
                        <button data-pw={1 + 1}>Count</button>
                    </>
                );
            }

            export function DynamicLink({ dataPw }) {
                return <a data-pw={dataPw}>Dynamic</a>;
            }

            export const ArrowLink = ({ dataPw = 'arrow-link' }) => {
                return <a data-pw={dataPw}>Arrow</a>;
            };

            export function DirectDefault(dataPw = 'direct-link') {
                return <a data-pw={dataPw}>Direct</a>;
            }

            export function ArrayDefault([dataPw = 'array-link']) {
                return <a data-pw={dataPw}>Array</a>;
            }

            export function NonStringDefault({ value = makeId() }) {
                return <a data-pw={value}>Computed</a>;
            }

            export function NestedShadow({ dataPw = 'outer-link' }) {
                function Inner({ dataPw }) {
                    return <a data-pw={dataPw}>Inner</a>;
                }
                return <Inner />;
            }

            export function Reassigned({ reassigned = 'assigned-link' }) {
                reassigned = makeId();
                return <a data-pw={reassigned}>Assigned</a>;
            }

            export function CompoundReassigned({ compound = 'compound-link' }) {
                compound += '-dynamic';
                return <a data-pw={compound}>Compound</a>;
            }

            export function DestructuredShadow({ shadowed = 'shadowed-link' }, props) {
                const { shadowed } = props;
                return <a data-pw={shadowed}>Shadowed</a>;
            }

            export function CommentAndStringText({ dataPw = 'comment-safe-link' }) {
                // dataPw = makeId();
                const message = "dataPw = makeId();";
                return <a data-pw={dataPw}>Comment safe</a>;
            }

            export function TemplateExpressionMutation({ mutated = 'template-mutation-link' }) {
                const label = `${mutated = makeId()}`;
                return <a data-pw={mutated}>Template mutation</a>;
            }

            export function EarlierHelperParam({ dataPw = 'helper-param-link' }) {
                function helper(dataPw) {
                    return dataPw;
                }
                const local = (dataPw) => dataPw;
                return <a data-pw={dataPw}>{helper(local('x'))}</a>;
            }

            export function WithHelper({ dataPw = 'helper-link' }) {
                const isReady = () => dataPw === 'helper-link';
                return isReady() ? <a data-pw={dataPw}>Ready</a> : null;
            }

            export function ShortName({ id = 'short-link' }) {
                const userId = makeId();
                return <a data-pw={id}>Short</a>;
            }
            "#,
            &attrs(),
            &component_attrs(),
        )
        .unwrap();

        let mut values: Vec<String> = selectors.iter().map(AppSelector::display_value).collect();
        values.sort();
        assert_eq!(
            values,
            vec![
                "array-link",
                "arrow-link",
                "comment-safe-link",
                "direct-link",
                "helper-link",
                "helper-param-link",
                "rss-feed-link",
                "short-link",
                "{1 + 1}",
                "{compound}",
                "{dataPw}",
                "{mutated}",
                "{passThrough}",
                "{reassigned}",
                "{shadowed}",
                "{value}",
            ]
        );
    }

    #[test]
    fn collect_app_selectors_reads_source_files_and_skips_build_dirs() {
        let root = fixture_path(&["selectors", "collect-app"]);
        let selectors = collect_app_selectors(&root, &attrs()).unwrap();
        assert_eq!(selectors.len(), 1);
        assert_eq!(selectors[0].display_value(), "ok");
        assert!(collect_app_selectors(&root.join("missing"), &attrs())
            .unwrap()
            .is_empty());

        let invalid = fixture_path(&["main", "invalid-selector-source", "web", "app"]);
        assert!(collect_app_selectors(&invalid, &attrs()).is_err());
    }

    #[test]
    fn extracts_playwright_css_and_test_id_selectors() {
        let source = fixture_source(&["selectors", "playwright-css-and-testid.ts"]);
        let selectors =
            extract_playwright_selectors(&source, &attrs(), &["data-testid".to_string()]);
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == "getByTestId(save)"));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == "[data-testid^='user-']"));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == r#"[data-pw$="button"]"#));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == r#"[data-pw*="nav"]"#));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == r#"[data-pw="exact"]"#));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == "getByTestId(/^account-/)"));
    }

    #[test]
    fn marks_selectors_inside_skipped_and_conditional_tests() {
        let selectors = extract_playwright_selector_occurrences(
            r#"
            test.skip('skipped', async ({ page }) => {
                await page.getByTestId('skipped');
            });
            test.fixme('fixme test', async ({ page }) => {
                await page.getByTestId('fixme');
            });
            if (process.env.E2E) {
                test('conditional wrapper', async ({ page }) => {
                    await page.getByTestId('conditional-wrapper');
                });
            } else {
                test('conditional alternate', async ({ page }) => {
                    await page.locator('[data-testid="conditional-alternate"]');
                });
            }
            featureFlag && test('logical wrapper', async ({ page }) => {
                await page.getByTestId('logical-wrapper');
            });
            featureFlag
                ? test('ternary consequent', async ({ page }) => {
                    await page.getByTestId('ternary-consequent');
                })
                : test('ternary alternate', async ({ page }) => {
                    await page.getByTestId('ternary-alternate');
                });
            test('active', async ({ page }) => {
                await page.getByTestId('active');
            });
            test.skip(({ browserName }) => browserName === 'webkit', 'conditional');
            test('file scope annotation', async ({ page }) => {
                await page.getByTestId('scope-annotation');
            });
            "#,
            &attrs(),
            &["data-testid".to_string()],
        );

        assert_eq!(
            selectors,
            vec![
                (
                    r#"[data-testid="conditional-alternate"]"#.to_string(),
                    TestStatus::Conditional
                ),
                ("getByTestId(active)".to_string(), TestStatus::Active),
                (
                    "getByTestId(conditional-wrapper)".to_string(),
                    TestStatus::Conditional
                ),
                ("getByTestId(fixme)".to_string(), TestStatus::Skipped),
                (
                    "getByTestId(logical-wrapper)".to_string(),
                    TestStatus::Conditional
                ),
                (
                    "getByTestId(scope-annotation)".to_string(),
                    TestStatus::Conditional
                ),
                ("getByTestId(skipped)".to_string(), TestStatus::Skipped),
                (
                    "getByTestId(ternary-alternate)".to_string(),
                    TestStatus::Conditional
                ),
                (
                    "getByTestId(ternary-consequent)".to_string(),
                    TestStatus::Conditional
                ),
            ]
        );
    }

    #[test]
    fn css_attribute_selectors_must_be_used_by_playwright_selector_calls() {
        let source = r#"
            const unused = '[data-testid="save"]';
            await page.locator('[data-testid="publish"]').click();
            await page.click(`[data-pw="open"]`);
            await page.type('[data-testid="search"]', 'query');
            await page.$eval('[data-pw="panel"]', node => node.textContent);
            await page.$$eval('[data-testid="items"]', nodes => nodes.length);
            await page.frameLocator('[data-pw="frame"]').locator('[data-testid="inside"]');
            await page.dragAndDrop('[data-testid="source"]', '[data-pw="target"]');
        "#;
        let selectors =
            extract_playwright_selectors(source, &attrs(), &["data-testid".to_string()]);
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == r#"[data-testid="publish"]"#));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == r#"[data-pw="open"]"#));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == r#"[data-testid="search"]"#));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == r#"[data-pw="panel"]"#));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == r#"[data-testid="items"]"#));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == r#"[data-pw="frame"]"#));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == r#"[data-testid="inside"]"#));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == r#"[data-testid="source"]"#));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == r#"[data-pw="target"]"#));
        assert!(selectors
            .iter()
            .all(|selector| selector.selector != r#"[data-testid="save"]"#));
    }

    #[test]
    fn selector_parser_handles_ast_edge_shapes() {
        let source = fixture_source(&["selectors", "edge-jsx.tsx"]);
        let selectors = extract_app_selectors(
            Path::new("app/page.tsx"),
            &source,
            &attrs(),
            &component_attrs(),
        )
        .unwrap();
        assert!(selectors
            .iter()
            .any(|selector| selector.display_value() == "save"));

        let source = fixture_source(&["selectors", "edge-playwright.ts"]);
        let selectors =
            extract_playwright_selectors(&source, &attrs(), &["data-testid".to_string()]);
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == "getByTestId(save)"));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == "getByTestId(publish)"));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == "getByTestId(wrapped-callee)"));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == "getByTestId(computed-receiver)"));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == "getByTestId(call-receiver)"));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == "getByTestId(optional-receiver)"));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == "getByTestId(optional-call)"));
        assert!(selectors
            .iter()
            .any(|selector| selector.selector == r#"[data-testid="save"]"#));
    }

    #[test]
    fn custom_test_id_attribute_maps_get_by_test_id() {
        let source = fixture_source(&["selectors", "custom-testid.ts"]);
        let selectors = extract_playwright_selectors(
            &source,
            &["data-test".to_string()],
            &["data-test".to_string()],
        );
        assert_eq!(selectors[0].attribute, "data-test");
    }

    #[test]
    fn exact_and_operator_matchers_cover_static_values() {
        let app = AppSelector {
            file: PathBuf::from("app/page.tsx"),
            attribute: "data-testid".to_string(),
            value: AppSelectorValue::Exact("save-button".to_string()),
        };
        let source = fixture_source(&["selectors", "exact-operator-matchers.ts"]);
        let selectors = extract_playwright_selectors(
            &source,
            &["data-testid".to_string()],
            &["data-testid".to_string()],
        );
        assert!(selectors
            .iter()
            .all(|selector| app.matches_playwright(selector)));
    }

    #[test]
    fn template_matchers_cover_structured_dynamic_values() {
        let app = AppSelector {
            file: PathBuf::from("app/page.tsx"),
            attribute: "data-testid".to_string(),
            value: AppSelectorValue::Template(TemplatePattern::new("user-${id}-button").unwrap()),
        };
        let source = fixture_source(&["selectors", "template-matchers.ts"]);
        let selectors = extract_playwright_selectors(
            &source,
            &["data-testid".to_string()],
            &["data-testid".to_string()],
        );
        assert!(selectors
            .iter()
            .all(|selector| app.matches_playwright(selector)));
    }

    #[test]
    fn mismatched_attributes_and_values_do_not_cover() {
        let app = AppSelector {
            file: PathBuf::from("app/page.tsx"),
            attribute: "data-testid".to_string(),
            value: AppSelectorValue::Exact("save".to_string()),
        };
        let source = fixture_source(&["selectors", "mismatched.ts"]);
        let selectors =
            extract_playwright_selectors(&source, &attrs(), &["data-testid".to_string()]);
        assert!(selectors
            .iter()
            .all(|selector| !app.matches_playwright(selector)));
    }

    #[test]
    fn unsupported_dynamic_values_never_match() {
        let source = fixture_source(&["selectors", "unsupported-dynamic.ts"]);
        let app = AppSelector {
            file: PathBuf::from("app/page.tsx"),
            attribute: "data-testid".to_string(),
            value: AppSelectorValue::Unsupported("id".to_string()),
        };
        let selectors = extract_playwright_selectors(
            &source,
            &["data-testid".to_string()],
            &["data-testid".to_string()],
        );
        assert!(!app.matches_playwright(&selectors[0]));
        assert_eq!(app.display_value(), "{id}");
    }

    #[test]
    fn unsupported_regex_selector_does_not_panic_or_match() {
        let app = AppSelector {
            file: PathBuf::from("app/page.tsx"),
            attribute: "data-testid".to_string(),
            value: AppSelectorValue::Exact("save".to_string()),
        };
        let selectors = extract_playwright_selectors(
            "await page.getByTestId(/(?<=prefix)save/);",
            &["data-testid".to_string()],
            &["data-testid".to_string()],
        );

        assert_eq!(selectors[0].selector, "getByTestId(/(?<=prefix)save/)");
        assert!(!app.matches_playwright(&selectors[0]));
    }

    #[test]
    fn playwright_selector_order_uses_matcher_kind_and_pattern() {
        let mut matchers = [
            SelectorMatcher::Contains("v".to_string()),
            SelectorMatcher::Regex {
                pattern: "^v".to_string(),
                compiled: Regex::new("^v").ok(),
            },
            SelectorMatcher::Suffix("v".to_string()),
            SelectorMatcher::Prefix("v".to_string()),
            SelectorMatcher::Exact("v".to_string()),
        ];

        assert_eq!(matchers[0], matchers[0]);
        matchers.sort();
        assert_eq!(
            matchers
                .iter()
                .map(SelectorMatcher::cmp_key)
                .collect::<Vec<_>>(),
            vec![(0, "v"), (1, "v"), (2, "v"), (3, "v"), (4, "^v")]
        );
    }

    #[test]
    fn malformed_template_is_treated_as_literal_pattern() {
        let pattern = TemplatePattern::new("user-${id").unwrap();
        assert!(pattern.matches_exact("user-${id"));
        assert_eq!(pattern.sample(), "user-${id");
    }

    #[test]
    fn template_without_static_parts_is_unsupported() {
        assert!(TemplatePattern::new("${id}").is_none());
    }

    #[test]
    fn template_exact_matching_rejects_non_matching_values() {
        let pattern = TemplatePattern::new("user-${id}-button").unwrap();
        assert!(!pattern.matches_exact("admin-1-button"));
        assert!(!pattern.matches_exact("user-1-link"));
        assert!(!pattern.matches_exact("user-1"));
        assert!(!pattern.matches_exact("user-button"));
    }

    #[test]
    fn identifier_reassignment_uses_identifier_boundaries_and_assignment_operator() {
        assert!(has_identifier_reassignment("dataPw = makeId();", "dataPw"));
        assert!(has_identifier_reassignment("dataPw += '-x';", "dataPw"));
        assert!(has_identifier_reassignment(
            "dataPw ??= makeId();",
            "dataPw"
        ));
        assert!(has_identifier_reassignment("dataPw++;", "dataPw"));
        assert!(has_identifier_reassignment("--dataPw;", "dataPw"));
        assert!(!has_identifier_reassignment("dataPw === 'save';", "dataPw"));
        assert!(!has_identifier_reassignment("dataPw == 'save';", "dataPw"));
        assert!(!has_identifier_reassignment(
            "// dataPw = makeId();\nconst message = \"dataPw += '-x';\";",
            "dataPw"
        ));
        assert!(!has_identifier_reassignment("userid = makeId();", "id"));
        assert!(!has_identifier_reassignment("id => id", "id"));
    }

    #[test]
    fn code_only_text_masks_comments_and_string_literals() {
        let masked = code_only_text(
            "const id = 'data\\'Pw';\n// dataPw = line\n/* dataPw = block\n*/ const text = \"data\\\"Pw\"; const tpl = `data ${dataPw = makeId({ nested: true })} \\`Pw`; dataPw += '-x';",
        );

        assert!(masked.contains("const id ="));
        assert!(masked.contains("const text ="));
        assert!(masked.contains("const tpl ="));
        assert!(masked.contains("dataPw = makeId({ nested: true })"));
        assert!(masked.contains("dataPw +="));
        assert!(!has_identifier_reassignment(
            "'dataPw = string';\n\"dataPw += string\";\n`dataPw++`;\n/* dataPw ??= block */",
            "dataPw"
        ));
    }

    #[test]
    fn enclosing_shadow_binding_requires_an_open_block() {
        let binding = Regex::new(r"\bfunction\b[^(]*\([^)]*\bdataPw\b").unwrap();

        assert!(has_enclosing_shadow_binding(
            "function Inner(dataPw) { return <a data-pw={",
            &binding
        ));
        assert!(has_enclosing_shadow_binding(
            "function Inner(dataPw) { if (ready) { dataPw; } return <a data-pw={",
            &binding
        ));
        assert!(!has_enclosing_shadow_binding(
            "function Inner(dataPw)",
            &binding
        ));
        assert!(!has_enclosing_shadow_binding(
            "function Inner(dataPw); return <a data-pw={",
            &binding
        ));
        assert!(!has_enclosing_shadow_binding(
            "function Inner(dataPw) { return dataPw; } return <a data-pw={",
            &binding
        ));
    }

    #[test]
    fn empty_internal_template_pattern_does_not_match() {
        let pattern = TemplatePattern {
            raw: String::new(),
            parts: vec![String::new()],
            starts_static: false,
            ends_static: false,
        };
        assert!(!pattern.matches_exact("anything"));
    }
}
