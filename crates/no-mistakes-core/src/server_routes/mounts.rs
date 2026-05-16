use crate::server_routes::model::{FileFacts, ImportBinding, RouteSite};
use crate::server_routes::normalize::join_paths;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const SOURCE_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx", "mjs", "mts", "cjs", "cts"];

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct ResolvedMount {
    pub parent_file: PathBuf,
    pub parent: String,
    pub child_file: PathBuf,
    pub child: String,
    pub prefix: String,
}

pub(crate) fn resolve_mounts(facts: &HashMap<PathBuf, FileFacts>) -> Vec<ResolvedMount> {
    let mut mounts = Vec::new();
    for (path, file_facts) in facts {
        for mount in &file_facts.mounts {
            if let Some((child_file, child)) = resolve_child(path, file_facts, &mount.child, facts)
            {
                mounts.push(ResolvedMount {
                    parent_file: path.clone(),
                    parent: mount.parent.clone(),
                    child_file,
                    child,
                    prefix: mount.prefix.clone(),
                });
            }
        }
    }
    mounts
}

pub(crate) fn prefixes_for(
    site: &RouteSite,
    facts: &HashMap<PathBuf, FileFacts>,
    mounts: &[ResolvedMount],
) -> Vec<String> {
    let own_prefixes = facts
        .get(&site.file)
        .and_then(|file| file.bindings.get(&site.binding))
        .map(|binding| binding.prefixes.clone())
        .filter(|prefixes| !prefixes.is_empty())
        .unwrap_or_else(|| vec![String::new()]);
    let mount_prefixes = mount_prefixes(&site.file, &site.binding, facts, mounts);
    let mount_prefixes = if mount_prefixes.is_empty() {
        vec![String::new()]
    } else {
        mount_prefixes
    };
    let mut prefixes = Vec::new();
    for mount_prefix in &mount_prefixes {
        for own_prefix in &own_prefixes {
            prefixes.push(join_paths(mount_prefix, own_prefix));
        }
    }
    prefixes.sort();
    prefixes.dedup();
    prefixes
}

fn resolve_child(
    path: &Path,
    file_facts: &FileFacts,
    child: &str,
    facts: &HashMap<PathBuf, FileFacts>,
) -> Option<(PathBuf, String)> {
    if file_facts.bindings.contains_key(child) {
        return Some((path.to_path_buf(), child.to_string()));
    }
    let import = file_facts
        .imports
        .iter()
        .find(|import| import.local == child)?;
    let target = resolve_import_path(path, &import.source, facts)?;
    let target_facts = facts.get(&target)?;
    let child = imported_binding(import, target_facts)?;
    Some((target, child))
}

fn imported_binding(import: &ImportBinding, facts: &FileFacts) -> Option<String> {
    if import.imported != "default"
        && (facts.exports.contains(&import.imported)
            || facts.bindings.contains_key(&import.imported))
    {
        return Some(import.imported.clone());
    }
    if facts.exports.contains(&import.local) || facts.bindings.contains_key(&import.local) {
        return Some(import.local.clone());
    }
    if facts.exports.len() == 1 {
        return facts.exports.iter().next().cloned();
    }
    None
}

fn resolve_import_path(
    from: &Path,
    source: &str,
    facts: &HashMap<PathBuf, FileFacts>,
) -> Option<PathBuf> {
    if !source.starts_with('.') {
        return None;
    }
    let base = from.parent()?.join(source);
    candidate_paths(&base)
        .into_iter()
        .filter_map(|candidate| candidate.canonicalize().ok().or(Some(candidate)))
        .find(|candidate| facts.contains_key(candidate))
}

fn candidate_paths(base: &Path) -> Vec<PathBuf> {
    if base.extension().is_some() {
        return vec![base.to_path_buf()];
    }
    let mut paths = SOURCE_EXTENSIONS
        .iter()
        .map(|ext| base.with_extension(ext))
        .collect::<Vec<_>>();
    paths.extend(
        SOURCE_EXTENSIONS
            .iter()
            .map(|ext| base.join("index").with_extension(ext)),
    );
    paths
}

fn mount_prefixes(
    file: &Path,
    binding: &str,
    facts: &HashMap<PathBuf, FileFacts>,
    mounts: &[ResolvedMount],
) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    collect_mount_prefixes(file, binding, "", facts, mounts, &mut seen, &mut out);
    out
}

fn collect_mount_prefixes(
    file: &Path,
    binding: &str,
    suffix: &str,
    facts: &HashMap<PathBuf, FileFacts>,
    mounts: &[ResolvedMount],
    seen: &mut HashSet<(PathBuf, String, String)>,
    out: &mut Vec<String>,
) {
    if !seen.insert((file.to_path_buf(), binding.to_string(), suffix.to_string())) {
        return;
    }
    for mount in mounts
        .iter()
        .filter(|mount| mount.child_file == file && mount.child == binding)
    {
        let prefix = join_paths(&mount.prefix, suffix);
        out.push(prefix.clone());
        if let Some(parent) = facts
            .get(&mount.parent_file)
            .and_then(|facts| facts.bindings.get(&mount.parent))
        {
            out.extend(
                parent
                    .prefixes
                    .iter()
                    .map(|parent_prefix| join_paths(parent_prefix, &prefix)),
            );
        }
        collect_mount_prefixes(
            &mount.parent_file,
            &mount.parent,
            &prefix,
            facts,
            mounts,
            seen,
            out,
        );
    }
}
