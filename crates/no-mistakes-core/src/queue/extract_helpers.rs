use crate::ast;
use oxc_ast::ast::{Argument, BindingPattern, Expression, ModuleExportName};
use oxc_span::GetSpan;
use std::collections::HashMap;

pub(super) fn binding_name(pattern: &BindingPattern<'_>) -> Option<String> {
    if let BindingPattern::BindingIdentifier(id) = pattern {
        return Some(id.name.as_str().to_string());
    }
    None
}

#[rustfmt::skip]
pub(super) fn export_name(name: &ModuleExportName<'_>) -> String {
    match name {
        ModuleExportName::IdentifierName(id) => id.name.as_str().to_string(), ModuleExportName::IdentifierReference(id) => id.name.as_str().to_string(),
        ModuleExportName::StringLiteral(value) => value.value.as_str().to_string(),
    }
}

pub(super) fn is_queue_package(source: &str) -> bool {
    matches!(source, "bullmq" | "glide-mq")
}

pub(super) fn literal_expr(
    expr: &Expression<'_>,
    consts: &HashMap<String, String>,
) -> Option<String> {
    match expr {
        Expression::StringLiteral(value) => Some(value.value.as_str().to_string()),
        Expression::Identifier(id) => consts.get(id.name.as_str()).cloned(),
        _ => None,
    }
}

pub(super) fn processor_specifier(
    args: &[Argument<'_>],
    source: &str,
    namespace_imports: &HashMap<String, String>,
) -> Option<String> {
    let source = args
        .iter()
        .filter_map(Argument::as_expression)
        .find_map(|expr| {
            let text = ast::span_text(source, expr.span());
            namespace_imports
                .keys()
                .find_map(|name| text.contains(name).then_some(name.clone()))
        });
    source.and_then(|name| namespace_imports.get(&name).cloned())
}

pub(super) fn collect_jobs_from_args(args: &[Argument<'_>], source: &str) -> Vec<String> {
    let text = args
        .iter()
        .map(|arg| ast::span_text(source, arg.span()))
        .collect::<Vec<_>>()
        .join("\n");
    let mut jobs = Vec::new();
    for marker in ["job.name ===", "job.name===", "name:"] {
        let mut rest = text.as_str();
        while let Some(index) = rest.find(marker) {
            rest = &rest[index + marker.len()..];
            let trimmed = rest.trim_start();
            if let Some(value) = quoted_prefix(trimmed) {
                jobs.push(value);
            }
        }
    }
    jobs
}

fn quoted_prefix(text: &str) -> Option<String> {
    let quote = text.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let rest = &text[quote.len_utf8()..];
    let mut escaped = false;
    let mut value = String::new();
    for ch in rest.chars() {
        if escaped {
            value.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            return Some(value);
        }
        value.push(ch);
    }
    None
}
