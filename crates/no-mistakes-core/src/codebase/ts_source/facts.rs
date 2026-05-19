use crate::codebase::dependencies::extract::{
    extract_imports_from_program, is_indexable, is_tsx_file, ExtractedImport,
};
use crate::codebase::ts_http_calls::{extract_http_calls_from_program, HttpCall};
use crate::codebase::ts_process_spawn::{extract_spawn_edges_from_program, SpawnEdge};
use crate::codebase::ts_queues::factory::{
    find_create_queue_line_from_program, find_queue_name_from_program,
};
use crate::codebase::ts_queues::usage::{extract_queue_usage_from_program, QueueUsage};
use crate::codebase::ts_routes::defs_backend::extract_backend_routes_from_program;
use crate::codebase::ts_routes::refs::{extract_route_refs_from_program, RouteRef};
use crate::codebase::ts_symbols::{extract_symbols_from_program, FileSymbols};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Default)]
pub struct TsFactPlan {
    pub imports: bool,
    pub symbols: bool,
    pub source: bool,
    pub route_refs: bool,
    pub backend_routes: bool,
    pub queue_usage: bool,
    pub queue_factory: bool,
    pub http_calls: bool,
    pub process_spawns: bool,
}

impl TsFactPlan {
    pub fn imports() -> Self {
        Self {
            imports: true,
            symbols: false,
            ..Self::default()
        }
    }

    pub fn imports_and_symbols() -> Self {
        Self {
            imports: true,
            symbols: true,
            ..Self::default()
        }
    }

    pub fn is_empty(self) -> bool {
        !self.imports
            && !self.symbols
            && !self.source
            && !self.route_refs
            && !self.backend_routes
            && !self.queue_usage
            && !self.queue_factory
            && !self.http_calls
            && !self.process_spawns
    }

    pub fn has_domain_facts(self) -> bool {
        self.source
            || self.route_refs
            || self.backend_routes
            || self.queue_usage
            || self.queue_factory
            || self.http_calls
            || self.process_spawns
    }
}

#[derive(Debug, Clone)]
pub struct TsFactContext {
    pub root: PathBuf,
    pub backend_register_object: Option<String>,
    pub queue_factory_specifier: Option<String>,
    pub queue_factory_function: Option<String>,
    pub http_prefixes: Vec<String>,
}

impl TsFactContext {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            ..Self::default()
        }
    }
}

impl Default for TsFactContext {
    fn default() -> Self {
        Self {
            root: PathBuf::new(),
            backend_register_object: None,
            queue_factory_specifier: None,
            queue_factory_function: None,
            http_prefixes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TsFileFacts {
    pub source: Option<String>,
    pub imports: Vec<ExtractedImport>,
    pub symbols: Option<FileSymbols>,
    pub route_refs: Vec<RouteRef>,
    pub backend_routes: Vec<(String, u32)>,
    pub queue_usage: Option<QueueUsage>,
    pub queue_create_line: Option<u32>,
    pub queue_name: Option<String>,
    pub http_calls: Vec<HttpCall>,
    pub process_spawns: Vec<SpawnEdge>,
}

pub type TsFactMap = HashMap<PathBuf, TsFileFacts>;

pub fn collect_ts_facts(files: &[PathBuf], plan: TsFactPlan) -> TsFactMap {
    collect_ts_facts_with_context(files, plan, &TsFactContext::default())
}

pub fn collect_ts_facts_with_context(
    files: &[PathBuf],
    plan: TsFactPlan,
    context: &TsFactContext,
) -> TsFactMap {
    files
        .par_iter()
        .filter(|path| is_indexable(path))
        .filter_map(|path| {
            collect_file_facts(path, plan, context).map(|facts| (path.clone(), facts))
        })
        .collect()
}

fn collect_file_facts(
    path: &Path,
    plan: TsFactPlan,
    context: &TsFactContext,
) -> Option<TsFileFacts> {
    let source = std::fs::read_to_string(path).ok()?;
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or_else(|_| {
        if is_tsx_file(path) {
            SourceType::tsx()
        } else {
            SourceType::ts()
        }
    });
    let parsed = Parser::new(&allocator, &source, source_type).parse();
    let imports = if plan.imports {
        extract_imports_from_program(&parsed.program)
    } else {
        Vec::new()
    };
    let symbols = plan
        .symbols
        .then(|| extract_symbols_from_program(&parsed.program, &source));
    let route_file = route_file_name(path, context);
    let route_refs = if plan.route_refs {
        extract_route_refs_from_program(&parsed.program, &source, &route_file)
    } else {
        Vec::new()
    };
    let backend_routes = if plan.backend_routes {
        context
            .backend_register_object
            .as_ref()
            .map(|register_object| {
                extract_backend_routes_from_program(&parsed.program, &source, register_object)
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let queue_usage = plan
        .queue_usage
        .then(|| extract_queue_usage_from_program(&parsed.program, &source));
    let (queue_create_line, queue_name) = if plan.queue_factory {
        match (
            context.queue_factory_specifier.as_deref(),
            context.queue_factory_function.as_deref(),
        ) {
            (Some(factory_specifier), Some(factory_function)) => (
                find_create_queue_line_from_program(
                    &parsed.program,
                    &source,
                    factory_specifier,
                    factory_function,
                ),
                find_queue_name_from_program(&parsed.program, factory_specifier, factory_function),
            ),
            _ => (None, None),
        }
    } else {
        (None, None)
    };
    let http_prefixes: Vec<&str> = context.http_prefixes.iter().map(String::as_str).collect();
    let http_calls = if plan.http_calls {
        extract_http_calls_from_program(&parsed.program, &source, &http_prefixes)
    } else {
        Vec::new()
    };
    let process_spawns = if plan.process_spawns {
        extract_spawn_edges_from_program(&parsed.program, &source, path, &context.root)
    } else {
        Vec::new()
    };
    Some(TsFileFacts {
        source: plan.source.then_some(source),
        imports,
        symbols,
        route_refs,
        backend_routes,
        queue_usage,
        queue_create_line,
        queue_name,
        http_calls,
        process_spawns,
    })
}

fn route_file_name(path: &Path, context: &TsFactContext) -> String {
    path.strip_prefix(&context.root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests;
