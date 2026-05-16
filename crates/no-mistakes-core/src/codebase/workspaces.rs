use anyhow::Result;
use globset::{Glob, GlobSetBuilder};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::codebase::ts_resolver::normalize_path;

/// A single NPM workspace package.
#[derive(Debug, Clone)]
pub struct WorkspacePackage {
    /// The `name` field from the package's `package.json`.
    pub name: String,
    /// Absolute path to the package directory.
    pub dir: PathBuf,
    /// Resolved absolute path to the package entry file, if any.
    pub entry: Option<PathBuf>,
    /// Raw `exports` field from package.json, used for exact and pattern subpath exports.
    pub exports: Option<serde_json::Value>,
}

/// All NPM workspace packages resolved from a root `package.json`.
#[derive(Debug, Default, Clone)]
pub struct WorkspaceMap {
    pub packages: Vec<WorkspacePackage>,
}

impl WorkspaceMap {
    /// Find a workspace package by name and return its entry file, if any.
    pub fn resolve_package(&self, name: &str) -> Option<&PathBuf> {
        self.packages
            .iter()
            .find(|p| p.name == name)
            .and_then(|p| p.entry.as_ref())
    }

    /// Resolve a bare workspace import specifier to the package entry or an exported subpath.
    pub fn resolve_specifier(&self, specifier: &str) -> Option<PathBuf> {
        let (name, subpath) = package_name_and_subpath(specifier)?;
        let package = self.packages.iter().find(|p| p.name == name)?;
        if subpath.is_none() {
            return package.entry.clone();
        }
        package.resolve_subpath(subpath.as_deref()?)
    }
}

impl WorkspacePackage {
    fn resolve_subpath(&self, subpath: &str) -> Option<PathBuf> {
        if let Some(exports) = &self.exports {
            return resolve_export_subpath(exports, subpath)
                .and_then(|target| try_resolve(&normalize_path(&self.dir.join(target))));
        }

        subpath
            .strip_prefix("./")
            .map(|relative| normalize_path(&self.dir.join(relative)))
            .and_then(|candidate| try_resolve(&candidate))
    }
}

#[derive(Deserialize, Default)]
struct PackageJson {
    name: Option<String>,
    workspaces: Option<WorkspacesField>,
    main: Option<String>,
    module: Option<String>,
    exports: Option<serde_json::Value>,
    types: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum WorkspacesField {
    Array(Vec<String>),
    Object { packages: Vec<String> },
}

#[derive(Deserialize, Default)]
struct PnpmWorkspace {
    packages: Option<Vec<String>>,
}

/// Load the workspace map from `root/package.json` or `root/pnpm-workspace.yaml`.
///
/// Returns an empty map if neither file declares workspaces.
pub fn load(root: &Path) -> Result<WorkspaceMap> {
    let workspace_globs = load_workspace_globs(root)?;
    let dirs = expand_workspace_globs(root, &workspace_globs);
    load_packages_from_dirs(dirs)
}

pub fn load_from_files(root: &Path, files: &[PathBuf]) -> Result<WorkspaceMap> {
    let workspace_globs = load_workspace_globs(root)?;
    let dirs = expand_workspace_globs_from_files(root, &workspace_globs, files);
    load_packages_from_dirs(dirs)
}

fn load_packages_from_dirs(dirs: Vec<PathBuf>) -> Result<WorkspaceMap> {
    let mut packages = Vec::new();
    for dir in dirs {
        if let Some(pkg) = load_package(&dir)? {
            packages.push(pkg);
        }
    }

    Ok(WorkspaceMap { packages })
}

pub fn load_workspace_globs(root: &Path) -> Result<Vec<String>> {
    let pnpm_path = root.join("pnpm-workspace.yaml");
    if pnpm_path.exists() {
        let content = std::fs::read_to_string(&pnpm_path)?;
        let pnpm_workspace: PnpmWorkspace = serde_yaml::from_str(&content)?;
        return Ok(pnpm_workspace
            .packages
            .unwrap_or_else(|| vec!["*".to_string()]));
    }

    let pkg_path = root.join("package.json");
    if pkg_path.exists() {
        let content = std::fs::read_to_string(&pkg_path)?;
        let root_pkg: PackageJson = serde_json::from_str(&content)?;

        let workspace_globs = match root_pkg.workspaces {
            Some(WorkspacesField::Array(globs)) => globs,
            Some(WorkspacesField::Object { packages }) => packages,
            None => Vec::new(),
        };
        return Ok(workspace_globs);
    }

    Ok(Vec::new())
}

fn build_glob_set(patterns: impl Iterator<Item = String>) -> Option<globset::GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let Ok(glob) = Glob::new(&pattern) else {
            continue;
        };
        let _ = builder.add(glob);
    }
    builder.build().ok()
}

fn expand_workspace_globs(root: &Path, glob_strs: &[String]) -> Vec<PathBuf> {
    let include = match build_glob_set(
        glob_strs
            .iter()
            .filter(|pattern| !pattern.starts_with('!'))
            .cloned(),
    ) {
        Some(g) => g,
        None => return vec![],
    };
    let exclude = build_glob_set(
        glob_strs
            .iter()
            .filter_map(|pattern| pattern.strip_prefix('!').map(String::from)),
    );

    let mut dirs = Vec::new();

    let glob_depth = glob_strs
        .iter()
        .filter(|pattern| !pattern.starts_with('!'))
        .map(|pattern| {
            if pattern.contains("**") {
                usize::MAX
            } else {
                pattern.split('/').count().max(1)
            }
        })
        .max()
        .unwrap_or(1);
    for entry in WalkDir::new(root)
        .min_depth(1)
        .max_depth(glob_depth)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
    {
        let rel = match entry.path().strip_prefix(root) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if include.is_match(rel) && !exclude.as_ref().is_some_and(|set| set.is_match(rel)) {
            dirs.push(entry.into_path());
        }
    }

    dirs
}

fn expand_workspace_globs_from_files(
    root: &Path,
    glob_strs: &[String],
    files: &[PathBuf],
) -> Vec<PathBuf> {
    let include = match build_glob_set(
        glob_strs
            .iter()
            .filter(|pattern| !pattern.starts_with('!'))
            .cloned(),
    ) {
        Some(g) => g,
        None => return vec![],
    };
    let exclude = build_glob_set(
        glob_strs
            .iter()
            .filter_map(|pattern| pattern.strip_prefix('!').map(String::from)),
    );

    let mut dirs: Vec<PathBuf> = files
        .iter()
        .filter(|path| path.file_name().and_then(|name| name.to_str()) == Some("package.json"))
        .filter_map(|path| path.parent())
        .filter_map(|dir| {
            let rel = dir.strip_prefix(root).ok()?;
            if include.is_match(rel) && !exclude.as_ref().is_some_and(|set| set.is_match(rel)) {
                Some(dir.to_path_buf())
            } else {
                None
            }
        })
        .collect();
    dirs.sort();
    dirs.dedup();
    dirs
}

fn load_package(dir: &Path) -> Result<Option<WorkspacePackage>> {
    let pkg_path = dir.join("package.json");
    if !pkg_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&pkg_path)?;
    let pkg: PackageJson = serde_json::from_str(&content).unwrap_or_default();

    let name = match pkg.name {
        Some(ref n) if !n.is_empty() => n.clone(),
        _ => return Ok(None),
    };

    // Resolve the entry file in priority order: exports > module > main > types
    let entry = resolve_entry(dir, &pkg);

    Ok(Some(WorkspacePackage {
        name,
        dir: dir.to_path_buf(),
        entry,
        exports: pkg.exports.clone(),
    }))
}

fn resolve_entry(dir: &Path, pkg: &PackageJson) -> Option<PathBuf> {
    // Check `exports` first (supports both string and `{".": ...}` forms).
    if let Some(exports) = &pkg.exports {
        if let Some(entry_str) = exports_to_entry_path(exports) {
            let p = normalize_path(&dir.join(&entry_str));
            if let Some(resolved) = try_resolve(&p) {
                return Some(resolved);
            }
        }
    }

    // module field (ESM)
    if let Some(module) = &pkg.module {
        let p = normalize_path(&dir.join(module));
        if let Some(resolved) = try_resolve(&p) {
            return Some(resolved);
        }
    }

    // main field (CJS/default)
    if let Some(main) = &pkg.main {
        let p = normalize_path(&dir.join(main));
        if let Some(resolved) = try_resolve(&p) {
            return Some(resolved);
        }
    }

    // types field
    if let Some(types) = &pkg.types {
        let p = normalize_path(&dir.join(types));
        if p.exists() {
            return Some(p);
        }
    }

    // Fallback: try common entry file names.
    for name in &[
        "src/index.mts",
        "src/index.ts",
        "src/index.tsx",
        "index.mts",
        "index.ts",
    ] {
        let p = dir.join(name);
        if p.exists() {
            return Some(p);
        }
    }

    None
}

fn exports_to_entry_path(exports: &serde_json::Value) -> Option<String> {
    match exports {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Object(map) => {
            if let Some(dot) = map.get(".") {
                return exports_to_entry_path(dot);
            }
            for key in &["import", "default", "require", "types"] {
                if let Some(v) = map.get(*key) {
                    if let Some(s) = exports_to_entry_path(v) {
                        return Some(s);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn resolve_export_subpath(exports: &serde_json::Value, subpath: &str) -> Option<String> {
    let serde_json::Value::Object(map) = exports else {
        return None;
    };

    if let Some(value) = map.get(subpath) {
        return exports_to_entry_path(value);
    }

    let mut patterns: Vec<_> = map
        .iter()
        .filter_map(|(pattern, value)| pattern.find('*').map(|star_idx| (pattern, value, star_idx)))
        .collect();
    patterns.sort_by(|(a, _, a_star), (b, _, b_star)| b_star.cmp(a_star).then_with(|| a.cmp(b)));

    for (pattern, value, star_idx) in patterns {
        if pattern[star_idx + 1..].contains('*') {
            continue;
        }
        let prefix = &pattern[..star_idx];
        let suffix = &pattern[star_idx + 1..];
        let Some(capture) = subpath
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_suffix(suffix))
        else {
            continue;
        };
        let Some(target) = exports_to_entry_path(value) else {
            continue;
        };
        if target.matches('*').count() == 1 {
            return Some(target.replacen('*', capture, 1));
        }
    }

    None
}

fn package_name_and_subpath(specifier: &str) -> Option<(String, Option<String>)> {
    if specifier.starts_with('.') || specifier.starts_with('/') {
        return None;
    }

    let mut parts = specifier.splitn(3, '/');
    let first = parts.next()?;
    if first.starts_with('@') {
        let scope_pkg = parts.next()?;
        let name_len = first.len() + 1 + scope_pkg.len();
        let subpath = specifier
            .get(name_len + 1..)
            .map(|rest| format!("./{rest}"));
        return Some((specifier[..name_len].to_string(), subpath));
    }

    let subpath = specifier
        .get(first.len() + 1..)
        .map(|rest| format!("./{rest}"));
    Some((first.to_string(), subpath))
}

fn try_resolve(path: &Path) -> Option<PathBuf> {
    if path.exists() {
        return Some(path.to_path_buf());
    }
    // Try appending TS extensions if no extension present.
    let s = path.to_string_lossy();
    for ext in &[".mts", ".ts", ".tsx", ".mjs", ".js", ".jsx"] {
        let candidate = PathBuf::from(format!("{s}{ext}"));
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests;
