use crate::ast;
use crate::queue::extract::{FileFacts, ProducerSite};
use crate::queue::extract_helpers::literal_expr;
use crate::queue::source::line_number;
use oxc_ast::ast::{Argument, CallExpression, ObjectPropertyKind};
use oxc_span::GetSpan;
use std::collections::HashMap;
use std::path::Path;

pub(super) fn record_enqueue(
    binding: &str,
    method: &str,
    path: &Path,
    source: &str,
    consts: &HashMap<String, String>,
    facts: &mut FileFacts,
    call: &CallExpression<'_>,
) {
    if method == "add" {
        facts.producers.push(ProducerSite {
            file: path.to_path_buf(),
            line: line_number(source, call.span.start),
            binding: binding.to_string(),
            raw_job: call.arguments.first().map(|arg| span(source, arg.span())),
            job: call
                .arguments
                .first()
                .and_then(|arg| arg.as_expression())
                .and_then(|expr| literal_expr(expr, consts)),
        });
        return;
    }
    if let Some(Argument::ArrayExpression(array)) = call.arguments.first() {
        for element in &array.elements {
            if let oxc_ast::ast::ArrayExpressionElement::ObjectExpression(object) = element {
                record_bulk_items(binding, path, source, consts, facts, call, object);
            }
        }
    }
}

pub(super) fn record_flow(
    path: &Path,
    source: &str,
    consts: &HashMap<String, String>,
    facts: &mut FileFacts,
    call: &CallExpression<'_>,
) {
    let Some(Argument::ObjectExpression(object)) = call.arguments.first() else {
        return;
    };
    let mut queue = None;
    let mut job = None;
    let mut raw_job = None;
    for property in &object.properties {
        if let ObjectPropertyKind::ObjectProperty(property) = property {
            match property.key.static_name().as_deref() {
                Some("queueName") => queue = literal_expr(&property.value, consts),
                Some("name") => {
                    raw_job = Some(span(source, property.value.span()));
                    job = literal_expr(&property.value, consts);
                }
                _ => {}
            }
        }
    }
    let Some(queue) = queue else {
        return;
    };
    let binding = format!("__flow__{queue}");
    facts.queue_bindings.insert(binding.clone(), queue.clone());
    facts.queue_exports.insert(binding.clone(), queue);
    facts.producers.push(ProducerSite {
        file: path.to_path_buf(),
        line: line_number(source, call.span.start),
        binding,
        raw_job,
        job,
    });
}

fn record_bulk_items(
    binding: &str,
    path: &Path,
    source: &str,
    consts: &HashMap<String, String>,
    facts: &mut FileFacts,
    call: &CallExpression<'_>,
    object: &oxc_ast::ast::ObjectExpression<'_>,
) {
    for property in &object.properties {
        if let ObjectPropertyKind::ObjectProperty(property) = property {
            if property.key.static_name().as_deref() == Some("name") {
                facts.producers.push(ProducerSite {
                    file: path.to_path_buf(),
                    line: line_number(source, call.span.start),
                    binding: binding.to_string(),
                    raw_job: Some(span(source, property.value.span())),
                    job: literal_expr(&property.value, consts),
                });
            }
        }
    }
}

fn span(source: &str, span: oxc_span::Span) -> String {
    ast::span_text(source, span).trim().to_string()
}
