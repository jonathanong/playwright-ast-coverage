#[allow(unused_imports)]
pub(crate) use no_mistakes_core::imports::{relative_string, resolve_import};

#[cfg(test)]
pub(crate) fn is_client_route_file(path: &std::path::Path) -> anyhow::Result<bool> {
    use no_mistakes_core::ast;

    if !path.exists() {
        return Ok(false);
    }

    let source = std::fs::read_to_string(path)?;
    ast::with_program(path, &source, |program, _| {
        program
            .directives
            .iter()
            .any(|directive| directive.directive == "use client")
    })
}
