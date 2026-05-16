use crate::fetch::types::FetchOccurrence;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct Cache {
    pub files: HashMap<(PathBuf, bool, bool), CachedFile>,
    pub imports: HashMap<PathBuf, Vec<PathBuf>>,
}

#[derive(Clone)]
pub struct CachedFile {
    pub is_client: bool,
    pub fetches: Vec<FetchOccurrence>,
}
