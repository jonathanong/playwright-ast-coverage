use crate::codebase::dependencies::extract::{
    extract_imports_from_program, is_indexable, is_tsx_file, ExtractedImport,
};
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
}

impl TsFactPlan {
    pub fn imports() -> Self {
        Self {
            imports: true,
            symbols: false,
        }
    }

    pub fn imports_and_symbols() -> Self {
        Self {
            imports: true,
            symbols: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TsFileFacts {
    pub imports: Vec<ExtractedImport>,
    pub symbols: Option<FileSymbols>,
}

pub type TsFactMap = HashMap<PathBuf, TsFileFacts>;

pub fn collect_ts_facts(files: &[PathBuf], plan: TsFactPlan) -> TsFactMap {
    files
        .par_iter()
        .filter(|path| is_indexable(path))
        .filter_map(|path| collect_file_facts(path, plan).map(|facts| (path.clone(), facts)))
        .collect()
}

fn collect_file_facts(path: &Path, plan: TsFactPlan) -> Option<TsFileFacts> {
    let source = std::fs::read_to_string(path).ok()?;
    let allocator = Allocator::default();
    let source_type = if is_tsx_file(path) {
        SourceType::tsx()
    } else {
        SourceType::ts()
    };
    let parsed = Parser::new(&allocator, &source, source_type).parse();
    let imports = if plan.imports {
        extract_imports_from_program(&parsed.program)
    } else {
        Vec::new()
    };
    let symbols = plan
        .symbols
        .then(|| extract_symbols_from_program(&parsed.program, &source));
    Some(TsFileFacts { imports, symbols })
}
