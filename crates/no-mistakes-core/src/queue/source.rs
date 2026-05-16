use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const SOURCE_EXTENSIONS: &[&str] = &["js", "jsx", "mjs", "mts", "cjs", "cts", "ts", "tsx"];

pub fn discover_source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let walker = WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !(entry.file_type().is_dir() && is_skip_dir(entry.path())));
    for entry in walker.filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if SOURCE_EXTENSIONS.contains(&ext) {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    files
}

pub fn relative_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn line_number(source: &str, start: u32) -> usize {
    source[..start as usize].lines().count() + 1
}

fn is_skip_dir(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
        matches!(
            n,
            ".git" | "node_modules" | "target" | "dist" | "build" | "coverage" | ".next"
        )
    })
}
