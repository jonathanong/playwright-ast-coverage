use crate::codebase::dependencies::extract::is_indexable;
use crate::codebase::ts_resolver::normalize_path;
use crate::codebase::ts_source::discover_files;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn discover(root: &Path, skip_directories: &[String]) -> HashSet<PathBuf> {
    discover_files(root, skip_directories)
        .into_iter()
        .filter(|path| is_indexable(path))
        .filter(|path| path.components().any(|c| c.as_os_str() == "__mocks__"))
        .flat_map(|path| mocked_targets(root, &path))
        .collect()
}

fn mocked_targets(root: &Path, mock_file: &Path) -> Vec<PathBuf> {
    let mut targets = Vec::new();
    if let Some(adjacent) = adjacent_target(mock_file) {
        targets.push(adjacent);
    }
    if let Some(rooted) = rooted_target(root, mock_file) {
        targets.push(rooted);
    }
    targets
}

fn adjacent_target(mock_file: &Path) -> Option<PathBuf> {
    let parent = mock_file.parent()?;
    if parent.file_name()?.to_str()? != "__mocks__" {
        return None;
    }
    Some(normalize_path(
        &parent.parent()?.join(mock_file.file_name()?),
    ))
}

fn rooted_target(root: &Path, mock_file: &Path) -> Option<PathBuf> {
    let rel = mock_file.strip_prefix(root).ok()?;
    let mut seen_mocks = false;
    let mut target = PathBuf::from(root);
    for component in rel.components() {
        if !seen_mocks {
            if component.as_os_str() == "__mocks__" {
                seen_mocks = true;
            }
            continue;
        }
        target.push(component.as_os_str());
    }
    seen_mocks.then(|| normalize_path(&target))
}

#[cfg(test)]
mod tests;
