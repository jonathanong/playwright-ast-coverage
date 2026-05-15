use super::helpers::{
    first_call_expression, first_statement_assignment_call_expression,
    object_argument_from_call_expression,
};
use crate::fetch::cache_opts::{extract_fetch_cache_options, infer_cached_wrapper_name};
use crate::report::types::CacheKind;

#[test]
fn test_extract_fetch_cache_options_cache_non_string() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "fetch('/api/no-store', { cache: true });";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_call_expression(&parsed.program.body[0]);
    let obj = object_argument_from_call_expression(call);
    let (cached, kind) = extract_fetch_cache_options(obj);
    assert!(!cached);
    assert_eq!(kind, CacheKind::None);
}

#[test]
fn test_extract_fetch_cache_options_next_unknown_property() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "fetch('/api/next', { next: { foo: true, revalidate: 0 } });";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_call_expression(&parsed.program.body[0]);
    let obj = object_argument_from_call_expression(call);
    let (cached, kind) = extract_fetch_cache_options(obj);
    assert!(!cached);
    assert_eq!(kind, CacheKind::None);
}

#[test]
fn test_extract_fetch_cache_options_next_non_object() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "fetch('/api/next', { next: 60 });";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_call_expression(&parsed.program.body[0]);
    let obj = object_argument_from_call_expression(call);
    let (cached, kind) = extract_fetch_cache_options(obj);
    assert!(!cached);
    assert_eq!(kind, CacheKind::None);
}

#[test]
fn test_extract_fetch_cache_options_next_computed_property_is_ignored() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "fetch('/api/next', { next: { [foo]: 60, revalidate: 60 } });";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_call_expression(&parsed.program.body[0]);
    let obj = object_argument_from_call_expression(call);
    let (cached, kind) = extract_fetch_cache_options(obj);
    assert!(cached);
    assert_eq!(kind, CacheKind::FetchNextRevalidate);
}

#[test]
fn test_extract_fetch_cache_options_force_cache() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "fetch('/api/cache', { cache: 'force-cache' });";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_call_expression(&parsed.program.body[0]);
    let obj = object_argument_from_call_expression(call);
    let (cached, kind) = extract_fetch_cache_options(obj);
    assert!(cached);
    assert_eq!(kind, CacheKind::FetchCache);
}

#[test]
fn test_extract_fetch_cache_options_next_revalidate() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "fetch('/api/next', { next: { revalidate: 60 } });";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_call_expression(&parsed.program.body[0]);
    let obj = object_argument_from_call_expression(call);
    let (cached, kind) = extract_fetch_cache_options(obj);
    assert!(cached);
    assert_eq!(kind, CacheKind::FetchNextRevalidate);
}

#[test]
fn test_extract_fetch_cache_options_tags() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "fetch('/api/tags', { next: { tags: ['alpha'] } });";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_call_expression(&parsed.program.body[0]);
    let obj = object_argument_from_call_expression(call);
    let (cached, kind) = extract_fetch_cache_options(obj);
    assert!(cached);
    assert_eq!(kind, CacheKind::FetchNextTags);
}

#[test]
fn test_infer_cached_wrapper_name_parses_cached_identifiers() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "cachedFn = cache(() => {});";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_statement_assignment_call_expression(&parsed.program.body[0]);
    assert_eq!(
        infer_cached_wrapper_name(source, call),
        Some("cachedFn".to_string())
    );
}

#[test]
fn test_infer_cached_wrapper_name_returns_none_for_direct_call() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "cache(() => {});";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_call_expression(&parsed.program.body[0]);
    assert_eq!(infer_cached_wrapper_name(source, call), None);
}

#[test]
fn test_infer_cached_wrapper_name_returns_none_when_text_after_equals() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "wrapped = /*cache helper*/ cache(() => {});";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_statement_assignment_call_expression(&parsed.program.body[0]);
    assert_eq!(infer_cached_wrapper_name(source, call), None);
}

#[test]
fn test_infer_cached_wrapper_name_returns_none_for_member_access_target() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "obj['value'] = cache(() => {});";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_statement_assignment_call_expression(&parsed.program.body[0]);
    assert_eq!(infer_cached_wrapper_name(source, call), None);
}

#[test]
fn test_infer_cached_wrapper_name_ignores_non_ascii_target() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "µcached = cache(() => {});";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_statement_assignment_call_expression(&parsed.program.body[0]);
    assert_eq!(infer_cached_wrapper_name(source, call), None);
}

#[test]
fn test_infer_cached_wrapper_name_returns_none_for_non_identifier_assignment_target() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "obj.cached_fn = cache(() => {});";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_statement_assignment_call_expression(&parsed.program.body[0]);
    assert_eq!(infer_cached_wrapper_name(source, call), None);
}

#[test]
fn test_infer_cached_wrapper_name_parses_multiline_assignment() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "
            cachedFn =\n            cache(() => {});\n        ";
    let source_type = oxc_span::SourceType::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let call = first_statement_assignment_call_expression(&parsed.program.body[0]);
    assert_eq!(
        infer_cached_wrapper_name(source, call),
        Some("cachedFn".to_string())
    );
}
