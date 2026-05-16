use std::path::{Path, PathBuf};

pub fn resolve_import(current_file: &Path, specifier: &str) -> Option<PathBuf> {
    const RUNTIME_EXTENSIONS: [&str; 4] = ["tsx", "ts", "jsx", "js"];

    if specifier.starts_with('.') {
        let parent = current_file.parent()?;
        let joined = parent.join(specifier);
        if joined.exists() && joined.is_file() {
            if !joined
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| RUNTIME_EXTENSIONS.contains(&ext))
            {
                return None;
            }
            return Some(joined);
        }
        for ext in RUNTIME_EXTENSIONS {
            let path = joined.with_extension(ext);
            if path.exists() {
                return Some(path);
            }
            let index = joined.join(format!("index.{ext}"));
            if index.exists() {
                return Some(index);
            }
        }
    }
    None
}

pub fn relative_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub fn is_client_route_file(path: &Path) -> anyhow::Result<bool> {
    use crate::ast;

    if !path.exists() {
        return Ok(false);
    }

    let source = std::fs::read_to_string(path)?;
    ast::with_program(path, &source, |program, _| {
        program
            .directives
            .iter()
            .any(|directive| directive.directive == "use client")
    })
}
