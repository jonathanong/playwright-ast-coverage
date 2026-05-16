use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(crate) struct ImportBinding {
    pub local: String,
    pub imported: String,
    pub source: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ProducerSite {
    pub file: PathBuf,
    pub line: usize,
    pub binding: String,
    pub job: Option<String>,
    pub raw_job: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct WorkerSite {
    pub file: PathBuf,
    pub line: usize,
    pub queue_name: Option<String>,
    pub jobs: Vec<String>,
    pub processor_specifier: Option<String>,
    pub processor_file: Option<PathBuf>,
    pub wildcard: bool,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct FileFacts {
    pub imports: Vec<ImportBinding>,
    pub queue_bindings: HashMap<String, String>,
    pub queue_exports: HashMap<String, String>,
    pub producers: Vec<ProducerSite>,
    pub workers: Vec<WorkerSite>,
    pub diagnostics: Vec<(usize, String)>,
}
