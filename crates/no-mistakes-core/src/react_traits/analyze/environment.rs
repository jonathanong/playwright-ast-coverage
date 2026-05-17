use oxc_ast::ast::Program;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileEnvironment {
    Server,
    Client,
    Unknown,
}

#[cfg(test)]
mod tests;

pub(crate) fn detect_file_environment(program: &Program<'_>) -> FileEnvironment {
    let has_use_server = program
        .directives
        .iter()
        .any(|d| d.directive == "use server");
    let has_use_client = program
        .directives
        .iter()
        .any(|d| d.directive == "use client");
    match (has_use_server, has_use_client) {
        (true, _) => FileEnvironment::Server,
        (_, true) => FileEnvironment::Client,
        _ => FileEnvironment::Unknown,
    }
}
