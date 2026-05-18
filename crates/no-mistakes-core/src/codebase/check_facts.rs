use crate::codebase::dependencies::extract::{
    extract_imports_from_program, is_indexable, ExtractedImport,
};
use crate::codebase::rules::test_no_unmocked_dynamic_imports::ast::TestFacts;
use crate::codebase::ts_symbols::{extract_symbols_from_program, FileSymbols};
use crate::integration_tests::types::FileAnalysis as IntegrationFileAnalysis;
use crate::queue::extract::FileFacts as QueueFileFacts;
use crate::react_traits::analyze::file::FileAnalysis as ReactFileAnalysis;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Default)]
pub struct CheckFactPlan {
    pub imports: bool,
    pub symbols: bool,
    pub react: bool,
    pub queue: bool,
    pub integration: bool,
    pub dynamic_imports: bool,
    pub source: bool,
}

#[derive(Default)]
pub struct CheckFactMap {
    pub(crate) files: Vec<PathBuf>,
    pub(crate) ts: HashMap<PathBuf, CheckFileFacts>,
    pub stats: CheckFactStats,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CheckFactStats {
    pub files_discovered: usize,
    pub files_parsed: usize,
    pub parse_errors: usize,
}

#[derive(Default)]
pub(crate) struct CheckFileFacts {
    pub source: Option<String>,
    pub imports: Vec<ExtractedImport>,
    pub symbols: Option<FileSymbols>,
    pub react: Option<ReactFileAnalysis>,
    pub queue: Option<QueueFileFacts>,
    pub integration: Option<IntegrationFileAnalysis>,
    pub dynamic_imports: Option<TestFacts>,
    pub parse_error: Option<String>,
}

impl CheckFactMap {
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    pub(crate) fn ts_facts(&self) -> crate::codebase::ts_source::facts::TsFactMap {
        let mut ts_facts = crate::codebase::ts_source::facts::TsFactMap::new();
        for (path, facts) in &self.ts {
            ts_facts.insert(
                path.clone(),
                crate::codebase::ts_source::facts::TsFileFacts {
                    imports: facts.imports.clone(),
                    symbols: facts.symbols.clone(),
                },
            );
        }
        ts_facts
    }
}

pub fn collect_check_facts(root: &Path, files: Vec<PathBuf>, plan: CheckFactPlan) -> CheckFactMap {
    let stats = CheckFactStats {
        files_discovered: files.len(),
        ..CheckFactStats::default()
    };
    let ts: HashMap<_, _> = files
        .par_iter()
        .filter(|path| is_indexable(path))
        .filter_map(|path| collect_file_facts(root, path, plan).map(|facts| (path.clone(), facts)))
        .collect();
    let mut files_parsed = 0;
    let mut parse_errors = 0;
    for facts in ts.values() {
        if facts.parse_error.is_none() {
            files_parsed += 1;
        } else {
            parse_errors += 1;
        }
    }
    CheckFactMap {
        files,
        ts,
        stats: CheckFactStats {
            files_parsed,
            parse_errors,
            ..stats
        },
    }
}

fn collect_file_facts(root: &Path, path: &Path, plan: CheckFactPlan) -> Option<CheckFileFacts> {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => {
            return Some(CheckFileFacts {
                parse_error: Some(format!("failed to read {}: {err}", path.display())),
                ..CheckFileFacts::default()
            });
        }
    };
    let source_type =
        SourceType::from_path(path).expect("indexable JavaScript/TypeScript extension");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &source, source_type).parse();
    if parsed.panicked || !parsed.errors.is_empty() {
        let parse_error = format!("{:?}", parsed.errors.first().expect("parser error details"));
        return Some(CheckFileFacts {
            source: plan.source.then_some(source),
            parse_error: Some(parse_error),
            ..CheckFileFacts::default()
        });
    }
    let program = &parsed.program;
    let imports = if plan.imports {
        extract_imports_from_program(program)
    } else {
        Vec::new()
    };
    let symbols = if plan.symbols {
        Some(extract_symbols_from_program(program, &source))
    } else {
        None
    };
    let react = if plan.react {
        Some(crate::react_traits::analyze::file::analyze_program(
            path, root, &source, program,
        ))
    } else {
        None
    };
    let queue = if plan.queue {
        Some(crate::queue::extract::extract_program(
            path, &source, program,
        ))
    } else {
        None
    };
    let integration = if plan.integration {
        Some(crate::integration_tests::analysis::analyze_program(
            path, program, &source,
        ))
    } else {
        None
    };
    let dynamic_imports = if plan.dynamic_imports {
        Some(
            crate::codebase::rules::test_no_unmocked_dynamic_imports::ast::extract_program(
                &source, program,
            ),
        )
    } else {
        None
    };
    Some(CheckFileFacts {
        source: plan.source.then_some(source),
        imports,
        symbols,
        react,
        queue,
        integration,
        dynamic_imports,
        parse_error: None,
    })
}

#[cfg(test)]
mod tests;
