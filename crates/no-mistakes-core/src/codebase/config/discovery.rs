use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::config::CONFIG_EXTENSIONS;

pub(super) fn find_codebase_config_path(start: &Path) -> Result<Option<PathBuf>> {
    let mut current = start.to_path_buf();
    loop {
        if let Some(path) = find_config_for_stem(&current, ".no-mistakes")? {
            return Ok(Some(path));
        }
        if let Some(path) = find_config_for_stem(&current, ".guardrailsrc")? {
            return Ok(Some(path));
        }
        if !current.pop() {
            return Ok(None);
        }
    }
}

fn find_config_for_stem(root: &Path, stem: &str) -> Result<Option<PathBuf>> {
    let found = CONFIG_EXTENSIONS
        .iter()
        .map(|ext| root.join(format!("{stem}.{ext}")))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    match found.len() {
        0 => Ok(None),
        1 => Ok(found.into_iter().next()),
        _ => {
            let files = found
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::bail!("multiple config files found under --root: {files}");
        }
    }
}
