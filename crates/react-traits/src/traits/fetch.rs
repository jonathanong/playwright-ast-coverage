use no_mistakes_core::fetch::types::FetchOccurrence;
use no_mistakes_core::fetch::visitor::FetchVisitor;
use oxc_ast::ast::Program;
use oxc_ast_visit::Visit;

pub(crate) fn collect_fetch_calls(
    program: &Program<'_>,
    source: &str,
    rel_file: &str,
) -> Vec<FetchOccurrence> {
    let mut visitor = FetchVisitor::new(source, rel_file, false, false);
    visitor.visit_program(program);
    visitor.fetches
}

#[cfg(test)]
mod tests;
