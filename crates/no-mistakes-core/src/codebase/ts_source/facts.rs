use crate::codebase::dependencies::extract::{
    extract_imports_from_program, is_indexable, ExtractedImport,
};
use crate::codebase::ts_http_calls::HttpCall;
use crate::codebase::ts_process_spawn::SpawnEdge;
use crate::codebase::ts_queues::usage::QueueUsage;
use crate::codebase::ts_routes::refs::RouteRef;
use crate::codebase::ts_symbols::{extract_symbols_from_program, FileSymbols};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod domain;
pub use domain::TsFactContext;

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
        self.route_refs
            || self.backend_routes
            || self.queue_usage
            || self.queue_factory
            || self.http_calls
            || self.process_spawns
    }
}

#[derive(Debug, Clone, Default)]
pub struct BackendRouteFact {
    pub register_object: String,
    pub route: String,
    pub line: u32,
}

#[derive(Debug, Clone, Default)]
pub struct TsFileFacts {
    pub source: Option<String>,
    pub imports: Vec<ExtractedImport>,
    pub symbols: Option<FileSymbols>,
    pub route_refs: Vec<RouteRef>,
    pub backend_routes: Vec<BackendRouteFact>,
    pub queue_usage: Option<QueueUsage>,
    pub queue_create_line: Option<u32>,
    pub queue_name: Option<String>,
    pub http_calls: Vec<HttpCall>,
    pub process_spawns: Vec<SpawnEdge>,
}

pub type TsFactMap = HashMap<PathBuf, TsFileFacts>;

pub fn collect_ts_facts(files: &[PathBuf], plan: TsFactPlan) -> TsFactMap {
    assert!(
        !plan.has_domain_facts(),
        "domain fact plans require collect_ts_facts_with_context"
    );
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
    let source_type = SourceType::from_path(path).unwrap_or_else(|_| SourceType::ts());
    let parsed = Parser::new(&allocator, &source, source_type).parse();
    let imports = if plan.imports {
        extract_imports_from_program(&parsed.program)
    } else {
        Vec::new()
    };
    let symbols = plan
        .symbols
        .then(|| extract_symbols_from_program(&parsed.program, &source));
    let domain = if plan.has_domain_facts() {
        domain::collect_domain_facts(&parsed.program, path, &source, plan, context)
    } else {
        domain::DomainFacts::default()
    };
    Some(TsFileFacts {
        source: plan.source.then_some(source),
        imports,
        symbols,
        route_refs: domain.route_refs,
        backend_routes: domain.backend_routes,
        queue_usage: domain.queue_usage,
        queue_create_line: domain.queue_create_line,
        queue_name: domain.queue_name,
        http_calls: domain.http_calls,
        process_spawns: domain.process_spawns,
    })
}

#[cfg(test)]
mod tests;
