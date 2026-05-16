use super::helpers::first_call_expression;
use crate::fetch::url_extract::{extract_url_from_argument, source_text};
use no_mistakes_core::fetch::types::UrlExtraction;

#[test]
fn test_extract_string_literal_from_argument_none() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "fetch(true)";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_call_expression(&parsed.program.body[0]);
    let arg = &call.arguments[0];
    assert_eq!(
        extract_url_from_argument(arg, source),
        UrlExtraction {
            path: "dynamic".to_string(),
            raw_path: "true".to_string(),
            is_dynamic: true,
            is_unsupported: true,
        }
    );
}

#[test]
fn test_extract_url_from_argument_works_for_direct_call_expression() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "fetch('/api/direct');";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_call_expression(&parsed.program.body[0]);
    let result = extract_url_from_argument(&call.arguments[0], source);
    assert_eq!(
        result,
        UrlExtraction {
            path: "/api/direct".to_string(),
            raw_path: "/api/direct".to_string(),
            is_dynamic: false,
            is_unsupported: false,
        }
    );
}

#[test]
fn test_extract_url_from_argument_works_for_nonnumeric_argument() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "fetch(123)";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_call_expression(&parsed.program.body[0]);
    let arg = &call.arguments[0];
    assert_eq!(
        extract_url_from_argument(arg, source),
        UrlExtraction {
            path: "dynamic".to_string(),
            raw_path: "123".to_string(),
            is_dynamic: true,
            is_unsupported: true,
        }
    );
}

#[test]
fn test_extract_url_from_argument_works_for_template_literal() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "fetch(`/api/foo`)";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_call_expression(&parsed.program.body[0]);
    let arg = &call.arguments[0];
    assert_eq!(
        extract_url_from_argument(arg, source),
        UrlExtraction {
            path: "/api/foo".to_string(),
            raw_path: "`/api/foo`".to_string(),
            is_dynamic: false,
            is_unsupported: false,
        }
    );
}

#[test]
fn test_extract_url_from_argument_template_literal_uses_fallback_for_invalid_source_slice() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "fetch(`/api/${dynamic}`)";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_call_expression(&parsed.program.body[0]);
    let arg = &call.arguments[0];

    let result = extract_url_from_argument(arg, "`");

    assert_eq!(
        result,
        UrlExtraction {
            path: "/api/${}".to_string(),
            raw_path: "dynamic".to_string(),
            is_dynamic: true,
            is_unsupported: true,
        }
    );
}

#[test]
fn test_extract_url_from_argument_uses_fallback_for_invalid_source_slice() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "fetch(url)";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_call_expression(&parsed.program.body[0]);
    let arg = &call.arguments[0];

    let result = extract_url_from_argument(arg, "");

    assert_eq!(
        result,
        UrlExtraction {
            path: "dynamic".to_string(),
            raw_path: "dynamic".to_string(),
            is_dynamic: true,
            is_unsupported: true,
        }
    );
}

#[test]
fn test_source_text_handles_invalid_slices() {
    assert!(source_text(1, 0, "abc").is_none());
    assert!(source_text(0, 4, "abc").is_none());
    assert!(source_text(1, 2, "é").is_none());
    assert_eq!(source_text(0, 2, "é"), Some("é".to_string()));
}
