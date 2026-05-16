use crate::analyze::imports::{
    collect_identifier_references, collect_runtime_imports_from_program,
};
use crate::analyze::resolve::relative_string;
use crate::fetch::visitor::FetchVisitor;
use crate::pipeline::cache::{Cache, CachedFile};
use crate::report::types::FetchOccurrence;
use anyhow::Result;
use no_mistakes_core::ast;
use oxc_ast_visit::Visit;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(crate) fn analyze_file(
    path: &Path,
    root: &Path,
    visited: &mut HashSet<(PathBuf, bool, bool)>,
    fetches: &mut Vec<FetchOccurrence>,
    cache: &mut Cache,
    inherited_is_client: bool,
    inherited_is_route_handler: bool,
) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    let abs_path = path
        .canonicalize()
        .expect("canonicalize succeeds since path.exists() was checked above");
    let visit_key = (
        abs_path.clone(),
        inherited_is_client,
        inherited_is_route_handler,
    );
    if visited.contains(&visit_key) {
        return Ok(false);
    }
    visited.insert(visit_key);

    let cache_key = (
        abs_path.clone(),
        inherited_is_client,
        inherited_is_route_handler,
    );
    if let Some(cached_fetches) = cache.files.get(&cache_key) {
        fetches.extend(cached_fetches.fetches.clone());
        return Ok(cached_fetches.is_client);
    }

    let source = std::fs::read_to_string(&abs_path)?;
    let rel_file = relative_string(root, &abs_path);

    let mut file_fetches = Vec::new();
    let is_client = ast::with_program(path, &source, |program, _| -> Result<bool> {
        let has_use_server_directive = program
            .directives
            .iter()
            .any(|directive| directive.directive == "use server");
        let has_use_client_directive = program
            .directives
            .iter()
            .any(|directive| directive.directive == "use client");
        let is_client = !inherited_is_route_handler
            && !has_use_server_directive
            && (inherited_is_client || has_use_client_directive);
        let mut visitor = FetchVisitor::new(
            &source,
            rel_file.as_str(),
            is_client,
            inherited_is_route_handler,
        );
        visitor.visit_program(program);
        file_fetches.extend(visitor.fetches);
        let referenced_identifiers = collect_identifier_references(program);
        let imports =
            collect_runtime_imports_from_program(&abs_path, program, &referenced_identifiers);
        for import in imports {
            match analyze_file(
                &import,
                root,
                visited,
                &mut file_fetches,
                cache,
                is_client,
                inherited_is_route_handler,
            ) {
                Ok(_) => {}
                Err(err) => return Err(err),
            }
        }
        Ok(is_client)
    })??;

    let cached = CachedFile {
        is_client,
        fetches: file_fetches.clone(),
    };
    cache.files.insert(cache_key.clone(), cached.clone());
    if cached.is_client != inherited_is_client {
        cache.files.insert(
            (abs_path.clone(), is_client, inherited_is_route_handler),
            cached,
        );
    }
    fetches.extend(file_fetches);

    Ok(is_client)
}
