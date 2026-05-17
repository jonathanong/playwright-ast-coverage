use crate::ast;
use oxc_ast::ast::{Expression, PropertyKey};

pub(in crate::integration_tests) fn property_key_name(key: &PropertyKey<'_>) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.to_string()),
        PropertyKey::StringLiteral(literal) => Some(literal.value.to_string()),
        _ => None,
    }
}

pub(in crate::integration_tests) fn optional_string(
    expression: &Expression<'_>,
    source: &str,
) -> Option<String> {
    match expression {
        Expression::StringLiteral(literal) => Some(literal.value.to_string()),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => {
            Some(ast::template_literal_text(template, source))
        }
        Expression::ParenthesizedExpression(parenthesized) => {
            optional_string(&parenthesized.expression, source)
        }
        _ => None,
    }
}

pub(in crate::integration_tests) fn required_string(
    expression: &Expression<'_>,
    source: &str,
    name: &str,
) -> anyhow::Result<String> {
    optional_string(expression, source)
        .ok_or_else(|| anyhow::anyhow!("expected string literal for {name}"))
}

pub(in crate::integration_tests) fn required_string_or_array(
    expression: &Expression<'_>,
    source: &str,
    name: &str,
) -> anyhow::Result<Vec<String>> {
    if let Some(value) = optional_string(expression, source) {
        return Ok(vec![value]);
    }
    let Expression::ArrayExpression(array) = parenthesized_expression(expression) else {
        anyhow::bail!("expected string literal or string array for {name}");
    };
    let mut values = Vec::new();
    for element in &array.elements {
        match element {
            oxc_ast::ast::ArrayExpressionElement::StringLiteral(literal) => {
                values.push(literal.value.to_string())
            }
            oxc_ast::ast::ArrayExpressionElement::TemplateLiteral(template)
                if template.expressions.is_empty() =>
            {
                values.push(ast::template_literal_text(template, source));
            }
            _ => anyhow::bail!("expected string literal array entries for {name}"),
        }
    }
    if values.is_empty() {
        anyhow::bail!("expected string literal or string array for {name}");
    }
    Ok(values)
}

fn parenthesized_expression<'a>(expression: &'a Expression<'a>) -> &'a Expression<'a> {
    match expression {
        Expression::ParenthesizedExpression(parenthesized) => {
            parenthesized_expression(&parenthesized.expression)
        }
        _ => expression,
    }
}
