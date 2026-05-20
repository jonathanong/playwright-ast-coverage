use oxc::allocator::Allocator;
use oxc::ast::ast::Program;
use oxc::parser::Parser;
use oxc::span::SourceType;

mod walk;

/// A detected HTTP client call with a literal path.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpCall {
    pub path: String,
    pub line: u32,
}

/// Extract HTTP method calls from `source` whose path starts with one of `prefixes`.
///
/// Detects:
/// - `<any>.<verb>('<path>', ...)` — chained method call with static first arg
/// - `fetch('<path>', ...)` — Fetch API with static first arg
///
/// Only calls whose path starts with one of `prefixes` are returned.
/// Template literals are accepted only when they have no interpolations; other
/// dynamic paths are skipped.
pub fn extract_http_calls(source: &str, prefixes: &[&str]) -> Vec<HttpCall> {
    let allocator = Allocator::default();
    let source_type = SourceType::tsx();
    let ret = Parser::new(&allocator, source, source_type).parse();
    extract_http_calls_from_program(&ret.program, source, prefixes)
}

pub fn extract_http_calls_from_program<'a>(
    program: &Program<'a>,
    source: &str,
    prefixes: &[&str],
) -> Vec<HttpCall> {
    walk::extract_http_calls_from_program(program, source, prefixes)
}

#[cfg(test)]
mod tests;
