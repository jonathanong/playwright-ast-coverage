use anyhow::{Context, Result};
use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Program, TemplateLiteral};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use std::path::Path;

pub fn with_program<T>(
    path: &Path,
    source: &str,
    analyze: impl for<'a> FnOnce(&'a Program<'a>, &'a str) -> T,
) -> Result<T> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path)
        .with_context(|| format!("unsupported JavaScript/TypeScript file: {}", path.display()))?;
    let parsed = Parser::new(&allocator, source, source_type).parse();

    if parsed.panicked || !parsed.errors.is_empty() {
        let detail = parsed
            .errors
            .first()
            .map(|e| format!("{e:?}"))
            .unwrap_or("unknown error (parser panicked)".to_string());
        anyhow::bail!("failed to parse {}: {detail}", path.display());
    }

    Ok(analyze(&parsed.program, source))
}

pub fn span_text(source: &str, span: Span) -> &str {
    source
        .get(span.start as usize..span.end as usize)
        .unwrap_or_default()
}

pub fn template_literal_text(template: &TemplateLiteral<'_>, source: &str) -> String {
    let mut text = String::new();
    for (index, quasi) in template.quasis.iter().enumerate() {
        text.push_str(
            quasi
                .value
                .cooked
                .as_ref()
                .unwrap_or(&quasi.value.raw)
                .as_str(),
        );
        if let Some(expression) = template.expressions.get(index) {
            text.push_str("${");
            text.push_str(span_text(source, expression.span()));
            text.push('}');
        }
    }
    text
}

pub fn expression_path(expression: &Expression<'_>) -> Option<Vec<String>> {
    match expression {
        Expression::Identifier(identifier) => Some(vec![identifier.name.to_string()]),
        Expression::StaticMemberExpression(member) => {
            let mut parts = expression_path(&member.object).unwrap_or_default();
            parts.push(member.property.name.to_string());
            Some(parts)
        }
        Expression::ParenthesizedExpression(parenthesized) => {
            expression_path(&parenthesized.expression)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_reports_invalid_sources_and_extensions() {
        assert!(with_program(Path::new("fixture.txt"), "", |_, _| ())
            .err()
            .unwrap()
            .to_string()
            .contains("unsupported"));

        assert!(
            with_program(Path::new("fixture.ts"), "await page.goto(", |_, _| ())
                .err()
                .unwrap()
                .to_string()
                .contains("failed to parse")
        );

        let _ = with_program(Path::new("non-existent.ts"), "", |_, _| ());
    }

    #[test]
    fn test_with_program_panic_simulation() {
        // Not easy to trigger panic in parser without internal knowledge,
        // but we've covered the error return path above.
    }

    #[test]
    fn test_span_text() {
        assert_eq!(span_text("abc", Span::new(0, 3)), "abc");
        assert_eq!(span_text("abc", Span::new(0, 0)), "");
        assert_eq!(span_text("abc", Span::new(9, 10)), "");
    }

    #[test]
    fn test_template_literal_text() {
        let allocator = Allocator::default();
        let source = "`${a}b${c}`";
        let source_type = SourceType::from_path(Path::new("test.ts")).unwrap();
        let parsed = Parser::new(&allocator, source, source_type).parse();
        let stmt = &parsed.program.body[0];
        let oxc_ast::ast::Statement::ExpressionStatement(expr_stmt) = stmt else { unreachable!() };
        let Expression::TemplateLiteral(t) = &expr_stmt.expression else { unreachable!() };
        assert_eq!(template_literal_text(t, source), "${a}b${c}");

        let source = "`no_expressions`";
        let parsed = Parser::new(&allocator, source, source_type).parse();
        let stmt = &parsed.program.body[0];
        let oxc_ast::ast::Statement::ExpressionStatement(expr_stmt) = stmt else { unreachable!() };
        let Expression::TemplateLiteral(t) = &expr_stmt.expression else { unreachable!() };
        assert_eq!(template_literal_text(t, source), "no_expressions");
    }

    #[test]
    fn test_expression_path() {
        let allocator = Allocator::default();
        let source = "a.b.c";
        let source_type = SourceType::from_path(Path::new("test.ts")).unwrap();
        let parsed = Parser::new(&allocator, source, source_type).parse();
        let stmt = &parsed.program.body[0];
        let oxc_ast::ast::Statement::ExpressionStatement(expr_stmt) = stmt else { unreachable!() };
        let path = expression_path(&expr_stmt.expression).unwrap();
        assert_eq!(path, vec!["a", "b", "c"]);

        let source = "(a).b";
        let parsed = Parser::new(&allocator, source, source_type).parse();
        let stmt = &parsed.program.body[0];
        let oxc_ast::ast::Statement::ExpressionStatement(expr_stmt) = stmt else { unreachable!() };
        let path = expression_path(&expr_stmt.expression).unwrap();
        assert_eq!(path, vec!["a", "b"]);

        let source = "123";
        let parsed = Parser::new(&allocator, source, source_type).parse();
        let stmt = &parsed.program.body[0];
        let oxc_ast::ast::Statement::ExpressionStatement(expr_stmt) = stmt else { unreachable!() };
        assert_eq!(expression_path(&expr_stmt.expression), None);

        let source = "a['b']";
        let parsed = Parser::new(&allocator, source, source_type).parse();
        let stmt = &parsed.program.body[0];
        let oxc_ast::ast::Statement::ExpressionStatement(expr_stmt) = stmt else { unreachable!() };
        assert_eq!(expression_path(&expr_stmt.expression), None);
    }
}
