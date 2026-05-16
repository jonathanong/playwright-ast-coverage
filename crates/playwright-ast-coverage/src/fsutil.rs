use anyhow::Result;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub(crate) fn build_globset(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = GlobBuilder::new(pattern).literal_separator(false).build()?;
        builder.add(glob);
    }
    Ok(builder.build()?)
}

pub(crate) fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !(entry.file_type().is_dir() && is_skipped_dir(entry.path())))
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .collect();
    files.sort();
    files
}

pub(crate) fn is_skipped_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name,
                ".git" | "node_modules" | "target" | "dist" | "build" | "coverage" | "test-results"
            )
        })
}

pub(crate) fn relative_string(root: &Path, path: &Path) -> String {
    slash_path(path.strip_prefix(root).unwrap_or(path))
}

pub(crate) fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub(crate) fn absolutize(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()
            .expect("current working directory must be accessible")
            .join(path))
    }
}
