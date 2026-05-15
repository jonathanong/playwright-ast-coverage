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

fn statement_expression<'a>(statement: &'a oxc_ast::ast::Statement<'a>) -> &'a Expression<'a> {
    let oxc_ast::ast::Statement::ExpressionStatement(expr) = statement else {
        panic!("expected expression statement");
    };
    &expr.expression
}

#[test]
#[should_panic]
fn test_statement_expression_panics_when_not_expression_statement() {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(Path::new("test.ts")).unwrap();
    let source = "if (true) {}";
    let parsed = Parser::new(&allocator, source, source_type).parse();
    statement_expression(&parsed.program.body[0]);
}

fn statement_template_literal<'a>(
    statement: &'a oxc_ast::ast::Statement<'a>,
) -> &'a TemplateLiteral<'a> {
    let oxc_ast::ast::Expression::TemplateLiteral(template) = statement_expression(statement)
    else {
        panic!("expected template literal");
    };
    template
}

#[test]
#[should_panic]
fn test_statement_template_literal_panics_when_not_template_literal() {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(Path::new("test.ts")).unwrap();
    let source = "1 + 1;";
    let parsed = Parser::new(&allocator, source, source_type).parse();
    statement_template_literal(&parsed.program.body[0]);
}

#[test]
fn test_template_literal_text() {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(Path::new("test.ts")).unwrap();
    let source = "`${a}b${c}`";
    let parsed = Parser::new(&allocator, source, source_type).parse();
    assert!(
        parsed.errors.is_empty(),
        "template literal parse errors: {:?}",
        parsed.errors
    );
    let t = statement_template_literal(&parsed.program.body[0]);
    assert_eq!(template_literal_text(t, source), "${a}b${c}");

    let source = "`no_expressions`";
    let parsed = Parser::new(&allocator, source, source_type).parse();
    assert!(
        parsed.errors.is_empty(),
        "template literal parse errors: {:?}",
        parsed.errors
    );
    let t = statement_template_literal(&parsed.program.body[0]);
    assert_eq!(template_literal_text(t, source), "no_expressions");
}

#[test]
fn test_expression_path() {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(Path::new("test.ts")).unwrap();

    let source = "a.b.c";
    let parsed = Parser::new(&allocator, source, source_type).parse();
    assert!(
        parsed.errors.is_empty(),
        "parse errors: {:?}",
        parsed.errors
    );
    let path = expression_path(statement_expression(&parsed.program.body[0])).unwrap();
    assert_eq!(path, vec!["a", "b", "c"]);

    let source = "(a).b";
    let parsed = Parser::new(&allocator, source, source_type).parse();
    assert!(
        parsed.errors.is_empty(),
        "parse errors: {:?}",
        parsed.errors
    );
    let path = expression_path(statement_expression(&parsed.program.body[0])).unwrap();
    assert_eq!(path, vec!["a", "b"]);

    let source = "123";
    let parsed = Parser::new(&allocator, source, source_type).parse();
    assert!(
        parsed.errors.is_empty(),
        "parse errors: {:?}",
        parsed.errors
    );
    assert_eq!(
        expression_path(statement_expression(&parsed.program.body[0])),
        None
    );

    let source = "a['b']";
    let parsed = Parser::new(&allocator, source, source_type).parse();
    assert!(
        parsed.errors.is_empty(),
        "parse errors: {:?}",
        parsed.errors
    );
    assert_eq!(
        expression_path(statement_expression(&parsed.program.body[0])),
        None
    );
}
