use anyhow::{Context, Result};
use oxc_allocator::Allocator;
use oxc_ast::ast::{Argument, CallExpression, Expression, ImportExpression, TemplateLiteral};
use oxc_ast_visit::{walk, Visit};
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::path::Path;

pub struct DynamicImport {
    pub specifier: Option<String>,
    pub line: usize,
}

#[derive(Default)]
pub struct TestFacts {
    pub dynamic_imports: Vec<DynamicImport>,
    pub mock_specifiers: Vec<String>,
}

pub fn extract(path: &Path, source: &str) -> Result<TestFacts> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).context(format!(
        "unsupported JavaScript/TypeScript file: {}",
        path.display()
    ))?;
    let parsed = Parser::new(&allocator, source, source_type).parse();
    let mut visitor = Collector {
        source,
        facts: TestFacts::default(),
    };
    visitor.visit_program(&parsed.program);
    Ok(visitor.facts)
}

struct Collector<'s> {
    source: &'s str,
    facts: TestFacts,
}

impl<'a> Visit<'a> for Collector<'_> {
    fn visit_import_expression(&mut self, import: &ImportExpression<'a>) {
        let line = crate::codebase::ts_source::byte_offset_to_line(
            self.source,
            import.span.start as usize,
        ) as usize;
        self.facts.dynamic_imports.push(DynamicImport {
            specifier: string_expr(&import.source),
            line,
        });
        walk::walk_import_expression(self, import);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if is_mock_call(call) {
            if let Some(first) = call.arguments.first() {
                if let Some(specifier) = string_arg(first) {
                    self.facts.mock_specifiers.push(specifier);
                }
            }
        }
        walk::walk_call_expression(self, call);
    }
}

fn is_mock_call(call: &CallExpression<'_>) -> bool {
    let Expression::StaticMemberExpression(member) = &call.callee else {
        return false;
    };
    let Expression::Identifier(object) = &member.object else {
        return false;
    };
    if !matches!(object.name.as_str(), "vi" | "jest") {
        return false;
    }
    matches!(
        member.property.name.as_str(),
        "mock" | "doMock" | "unstable_mockModule" | "setMock"
    )
}

fn string_arg(arg: &Argument<'_>) -> Option<String> {
    match arg {
        Argument::StringLiteral(s) => Some(s.value.as_str().to_string()),
        Argument::TemplateLiteral(t) => static_template(t),
        _ => None,
    }
}

fn string_expr(expr: &Expression<'_>) -> Option<String> {
    match crate::codebase::ts_source::unwrap_ts_wrappers(expr) {
        Expression::StringLiteral(s) => Some(s.value.as_str().to_string()),
        Expression::TemplateLiteral(t) => static_template(t),
        _ => None,
    }
}

fn static_template(template: &TemplateLiteral<'_>) -> Option<String> {
    if !template.expressions.is_empty() {
        return None;
    }
    let mut value = String::new();
    for quasi in &template.quasis {
        value.push_str(quasi.value.cooked.as_ref().unwrap_or(&quasi.value.raw));
    }
    Some(value)
}

#[cfg(test)]
mod tests;
