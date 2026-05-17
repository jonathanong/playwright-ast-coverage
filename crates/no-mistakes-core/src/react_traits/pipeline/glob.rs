use anyhow::Result;
use globset::{GlobBuilder, GlobSetBuilder};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub(crate) fn expand_globs(root: &Path, patterns: &[String]) -> Result<Vec<PathBuf>> {
    if patterns.is_empty() {
        return Ok(Vec::new());
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = GlobBuilder::new(pattern).literal_separator(false).build()?;
        builder.add(glob);
    }
    let globset = builder.build()?;

    const EXTENSIONS: &[&str] = &["tsx", "ts", "jsx", "js"];
    let mut files = Vec::new();
    let walker = WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| !(e.file_type().is_dir() && is_skip_dir(e.path())));
    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !EXTENSIONS.contains(&ext) {
            continue;
        }
        let rel = path.strip_prefix(root).unwrap_or(path);
        if globset.is_match(rel) {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn is_skip_dir(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
        matches!(
            n,
            ".git" | "node_modules" | "target" | "dist" | "build" | "coverage"
        )
    })
}
