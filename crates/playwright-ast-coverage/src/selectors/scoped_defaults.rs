use super::shadowing::identifier_may_be_shadowed_or_reassigned;
use super::types::{AppSelectorValue, TemplatePattern};
use crate::ast;
use oxc_ast_visit::Visit;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;

pub(super) struct ScopedStaticIdentifierDefault {
    pub(super) name: String,
    pub(super) value: String,
    pub(super) scope: Span,
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

pub(super) fn collect_scoped_static_identifier_defaults(
    program: &oxc_ast::ast::Program<'_>,
) -> Vec<ScopedStaticIdentifierDefault> {
    let mut visitor = ScopedDefaultVisitor {
        defaults: Vec::new(),
    };
    visitor.visit_program(program);
    visitor.defaults
}

pub(super) fn collect_static_defaults_from_binding(
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

pub(super) fn scoped_static_default_for_identifier(
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

pub(super) fn jsx_attribute_name<'a>(
    name: &'a oxc_ast::ast::JSXAttributeName<'a>,
) -> Option<&'a str> {
    match name {
        oxc_ast::ast::JSXAttributeName::Identifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

pub(super) fn app_selector_value(
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

pub(super) fn jsx_expression_value(
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
