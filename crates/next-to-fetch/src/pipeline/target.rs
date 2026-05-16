use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub(crate) struct TargetSpec {
    pub(crate) raw: String,
    pub(crate) file: Option<PathBuf>,
}

pub(crate) fn resolve_target_file(root: &Path, target: &str) -> Result<PathBuf> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        anyhow::bail!("target path cannot be empty");
    }

    let candidate = if Path::new(trimmed).is_absolute() {
        Path::new(trimmed).to_path_buf()
    } else {
        root.join(trimmed)
    };
    if !candidate.is_file() {
        anyhow::bail!("target path is not a file: {}", candidate.display());
    }
    Ok(candidate
        .canonicalize()
        .expect("canonicalize succeeds since we verified the file exists above"))
}

pub(crate) fn normalize_target_pattern(target: &str) -> Option<String> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    })
}

pub(crate) fn route_matches_target(route_pattern: &str, target_raw: &str) -> bool {
    let Some(normalized_target) = normalize_target_pattern(target_raw) else {
        return false;
    };

    if normalized_target == "/" {
        return route_pattern == "/";
    }

    if normalized_target.ends_with('/') {
        let prefix = format!("{}/", normalized_target.trim_end_matches('/'));
        return route_pattern.starts_with(&prefix);
    }

    route_pattern == normalized_target
}
