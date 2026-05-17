use std::path::{Path, PathBuf};

pub fn discover_source_files(root: &Path) -> Vec<PathBuf> {
    crate::codebase::ts_source::discover_source_files(root, &[])
}

pub fn relative_string(root: &Path, path: &Path) -> String {
    crate::codebase::ts_source::relative_slash_path(root, path)
}

pub(crate) fn line_number(source: &str, start: u32) -> usize {
    crate::codebase::ts_source::line_number(source, start)
}
