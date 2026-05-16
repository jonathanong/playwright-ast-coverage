use crate::analyze::components::extract_components;
use crate::analyze::environment::{detect_file_environment, FileEnvironment};
use crate::analyze::import_table::build_import_table;
use crate::analyze::jsx_children::collect_jsx_children;
use crate::report::types::{ComponentFacts, ComponentRef, Environment, FetchCall};
use crate::traits;
use anyhow::Result;
use no_mistakes_core::ast;
use no_mistakes_core::imports::{
    collect_identifier_references, collect_runtime_imports_from_program, relative_string,
};
use std::path::{Path, PathBuf};

pub(crate) struct FileAnalysis {
    pub(crate) components: Vec<ComponentFacts>,
    #[allow(dead_code)]
    pub(crate) dependencies: Vec<PathBuf>,
}

#[cfg(test)]
mod tests;

pub(crate) fn analyze_file(abs_path: &Path, root: &Path) -> Result<FileAnalysis> {
    let source = std::fs::read_to_string(abs_path)?;
    let rel_path = relative_string(root, abs_path);

    let (components, dependencies) = ast::with_program(abs_path, &source, |program, _src| {
        let env = detect_file_environment(program);
        let import_table = build_import_table(abs_path, program);
        let component_defs = extract_components(program);

        let referenced = collect_identifier_references(program);
        let deps = collect_runtime_imports_from_program(abs_path, program, &referenced);

        let environment = match env {
            FileEnvironment::Server => Environment::Server,
            FileEnvironment::Client => Environment::Client,
            FileEnvironment::Unknown => Environment::Unknown,
        };

        let dep_strings: Vec<String> = deps.iter().map(|p| relative_string(root, p)).collect();

        let mut components = Vec::new();
        for def in component_defs {
            let span = def.span;
            let has_state = traits::state::detect_has_state(program, span);
            let (has_props, passes_props) = traits::props::detect_props(program, span);
            let uses_memo = traits::memo::detect_uses_memo(program, span, &def);
            let uses_context_provider = traits::context::detect_context_provider(program, span);
            let uses_suspense = traits::suspense::detect_uses_suspense(program, span);
            let fetch_calls = traits::fetch::collect_fetch_calls(program, &source, &rel_path, span);

            let fetches = fetch_calls
                .into_iter()
                .map(|f| FetchCall {
                    file: f.file.clone(),
                    exported_name: f.cached_function.clone(),
                    shape: Some(format!("{} {}", f.method, f.path)),
                })
                .collect();

            let children: Vec<ComponentRef> =
                collect_jsx_children(program, &import_table, span)
                    .into_iter()
                    .map(|(path, name)| ComponentRef {
                        name,
                        file: relative_string(root, &path),
                    })
                    .collect();

            components.push(ComponentFacts {
                name: def.name.clone(),
                file: rel_path.clone(),
                environment: environment.clone(),
                has_state,
                has_props,
                passes_props,
                uses_memo,
                uses_context_provider,
                uses_suspense,
                fetches,
                dependencies: dep_strings.clone(),
                children,
                inherited_from_children: None,
            });
        }

        (components, deps)
    })?;

    Ok(FileAnalysis {
        components,
        dependencies,
    })
}
