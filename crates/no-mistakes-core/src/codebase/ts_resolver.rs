use anyhow::{Context, Result};
use dashmap::DashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Parsed content of a `tsconfig.json` relevant to import resolution.
#[derive(Debug, Clone, Default)]
pub struct TsConfig {
    /// Directory containing the entry tsconfig (used as base for relative-path resolution).
    pub dir: PathBuf,
    /// `compilerOptions.paths` entries: glob pattern → list of replacement templates.
    pub paths: Vec<(String, Vec<String>)>,
    /// Directory of the tsconfig that *defines* `paths`. May differ from `dir` when the
    /// entry tsconfig inherits `paths` via `extends`. Used to anchor alias substitution.
    pub paths_dir: PathBuf,
    /// Absolute directory used for TypeScript `baseUrl` bare-specifier resolution.
    pub base_url: Option<PathBuf>,
}

/// Load and parse a `tsconfig.json` at `path`, following `extends` chains.
///
/// `compilerOptions.paths` is inherited from the nearest ancestor that defines it;
/// a child config that defines its own `paths` takes full precedence (no merge).
/// The directory of whichever config physically contains the winning `paths` block
/// is stored in `TsConfig::paths_dir` so that alias substitution is anchored there.
pub fn load_tsconfig(path: &Path) -> Result<TsConfig> {
    Ok(load_tsconfig_inner(path, &mut std::collections::HashSet::new())?.inner)
}

/// Internal result that carries whether `paths` was *explicitly defined* (even if empty)
/// somewhere in the extends chain. This lets the extends array logic correctly apply
/// TS override semantics: a later entry that defines `paths: {}` must win over an
/// earlier entry that defines `paths: { ... }`, even though the resolved vec is empty.
struct TsConfigFound {
    inner: TsConfig,
    /// `true` when `compilerOptions.paths` was present in the JSON of this config
    /// or of some config it inherited from. `false` means "not defined anywhere".
    paths_found: bool,
}

fn load_tsconfig_inner(
    path: &Path,
    visited: &mut std::collections::HashSet<PathBuf>,
) -> Result<TsConfigFound> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("resolving {}", path.display()))?;

    if !visited.insert(canonical.clone()) {
        anyhow::bail!("tsconfig.extends cycle detected at {}", path.display());
    }

    let dir = path
        .parent()
        .context(format!("resolving parent directory for {}", path.display()))?
        .to_path_buf();

    let content = std::fs::read_to_string(path).context(format!("reading {}", path.display()))?;

    let v: serde_json::Value =
        serde_json::from_str(&content).context(format!("parsing {}", path.display()))?;

    let own_paths: Option<Vec<(String, Vec<String>)>> = v
        .get("compilerOptions")
        .and_then(|co| co.get("paths"))
        .and_then(|p| p.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(pattern, replacements)| {
                    let repls = replacements
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default();
                    (pattern.clone(), repls)
                })
                .collect()
        });
    let own_base_url = v
        .get("compilerOptions")
        .and_then(|co| co.get("baseUrl"))
        .and_then(|p| p.as_str())
        .map(|base_url| normalize_path(&dir.join(base_url)));

    // Check for an extends chain.
    // `extends` may be a string (pre-TS 5.0) or an array of strings (TS 5.0+).
    // For arrays, TS 5.0 applies them left-to-right; the rightmost definition wins
    // for any given property. `paths` and `baseUrl` inherit independently, so a
    // later config that only defines `baseUrl` must not discard earlier `paths`.
    let extends_list: Vec<&str> = match v.get("extends") {
        None => vec![],
        Some(serde_json::Value::String(s)) => vec![s.as_str()],
        Some(serde_json::Value::Array(arr)) => {
            let mut list = Vec::with_capacity(arr.len());
            for item in arr {
                let s = item.as_str().ok_or_else(|| {
                    anyhow::anyhow!(
                        "{} extends array contains a non-string entry; \
                         TypeScript rejects non-string extends values",
                        path.display()
                    )
                })?;
                list.push(s);
            }
            list
        }
        Some(other) => anyhow::bail!(
            "{} extends must be a string or array of strings, got: {}; \
             TypeScript rejects this configuration",
            path.display(),
            other
        ),
    };

    let mut inherited_paths: Option<TsConfigFound> = None;
    let mut inherited_base_url: Option<PathBuf> = None;
    if !extends_list.is_empty() {
        for extends_raw in &extends_list {
            // Bare specifiers (npm packages like "@scope/tsconfig") cannot be resolved
            // without a node_modules lookup. Emit a warning and skip — the caller may
            // have local paths defined, or another entry in the extends array may provide
            // them. If alias resolution is unexpectedly empty, this warning identifies why.
            if !extends_raw.starts_with('.') && !std::path::Path::new(extends_raw).is_absolute() {
                eprintln!(
                    "warning: {} extends npm package '{}'; \
                     path aliases from that package cannot be resolved without node_modules",
                    path.display(),
                    extends_raw
                );
                continue;
            }
            let base_path = normalize_path(&dir.join(extends_raw));
            let base_path = if base_path.is_dir() {
                base_path.join("tsconfig.json")
            } else if base_path.extension().is_none() {
                base_path.with_extension("json")
            } else {
                base_path
            };
            let base = load_tsconfig_inner(&base_path, visited)
                .context(format!("loading extended tsconfig {}", base_path.display()))?;
            if let Some(base_url) = &base.inner.base_url {
                inherited_base_url = Some(base_url.clone());
            }
            if base.paths_found {
                inherited_paths = Some(base);
            }
        }
    }

    let base_url = own_base_url.or(inherited_base_url);

    // Child defines its own paths (even if empty). An explicit `paths: {}`
    // overrides any paths from the extends chain, but not an inherited baseUrl.
    if let Some(paths) = own_paths {
        let paths_dir = dir.clone();
        visited.remove(&canonical);
        return Ok(TsConfigFound {
            inner: TsConfig {
                dir,
                paths,
                paths_dir,
                base_url,
            },
            paths_found: true,
        });
    }

    if let Some(base) = inherited_paths {
        visited.remove(&canonical);
        return Ok(TsConfigFound {
            inner: TsConfig {
                dir,
                paths: base.inner.paths,
                paths_dir: base.inner.paths_dir,
                base_url,
            },
            paths_found: true,
        });
    }

    visited.remove(&canonical);
    Ok(TsConfigFound {
        inner: TsConfig {
            dir: dir.clone(),
            paths: vec![],
            paths_dir: dir,
            base_url,
        },
        paths_found: false,
    })
}

/// Walk parent directories from `start` looking for `tsconfig.json`.
pub fn find_tsconfig(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        let candidate = current.join("tsconfig.json");
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}

const EXTENSIONS: &[&str] = &[".mts", ".ts", ".tsx", ".mjs", ".js", ".jsx", ".cjs", ".cts"];
const EXPLICIT_EXTENSIONS: &[&str] = &[
    "mts", "ts", "tsx", "mjs", "js", "jsx", "cjs", "cts", "json", "css", "scss", "sass", "less",
    "svg", "png", "jpg", "jpeg", "gif", "webp", "avif", "txt", "wasm",
];

fn has_explicit_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| EXPLICIT_EXTENSIONS.contains(&ext))
        .unwrap_or(false)
}

/// Resolve `specifier` (as it appears in an import in `importing_file`) to an
/// absolute path on disk. Returns `None` for bare npm specifiers or if no file
/// is found.
///
/// Resolution order:
/// 1. Relative (`./` or `../`): join with importer's directory, try extension candidates.
/// 2. tsconfig path alias: match against `paths` map, substitute capture, try candidates.
/// 3. None.
pub fn resolve_import(
    specifier: &str,
    importing_file: &Path,
    tsconfig: &TsConfig,
) -> Option<PathBuf> {
    ImportResolver::new(tsconfig)
        .without_cache()
        .resolve(specifier, importing_file)
}

/// Cached resolver for batches of import lookups against one `TsConfig`.
pub struct ImportResolver<'a> {
    tsconfig: &'a TsConfig,
    visible: Option<&'a HashSet<PathBuf>>,
    alias_order: Vec<usize>,
    cache_enabled: bool,
    cache: DashMap<ResolveKey, Option<PathBuf>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ResolveKey {
    importing_dir: PathBuf,
    specifier: String,
}

impl<'a> ImportResolver<'a> {
    pub fn new(tsconfig: &'a TsConfig) -> Self {
        let mut alias_order: Vec<usize> = (0..tsconfig.paths.len()).collect();
        alias_order.sort_by(|&a, &b| {
            let la = tsconfig.paths[a].0.len();
            let lb = tsconfig.paths[b].0.len();
            lb.cmp(&la).then(a.cmp(&b))
        });

        Self {
            tsconfig,
            visible: None,
            alias_order,
            cache_enabled: true,
            cache: DashMap::new(),
        }
    }

    pub fn with_visible(mut self, visible: &'a HashSet<PathBuf>) -> Self {
        self.visible = Some(visible);
        self.cache_enabled = false;
        self
    }

    pub fn without_cache(mut self) -> Self {
        self.cache_enabled = false;
        self
    }

    pub fn resolve(&self, specifier: &str, importing_file: &Path) -> Option<PathBuf> {
        if !self.cache_enabled {
            return self.resolve_uncached(specifier, importing_file);
        }

        let importing_dir = importing_file.parent().map(normalize_path)?;
        let key = ResolveKey {
            importing_dir,
            specifier: specifier.to_string(),
        };

        if let Some(cached) = self.cache.get(&key) {
            return cached.clone();
        }
        let resolved = self.resolve_uncached(specifier, importing_file);
        self.cache.insert(key, resolved.clone());
        resolved
    }

    fn resolve_uncached(&self, specifier: &str, importing_file: &Path) -> Option<PathBuf> {
        if specifier.starts_with("./") || specifier.starts_with("../") {
            let dir = importing_file.parent()?;
            return self.try_path(&dir.join(specifier));
        }

        for idx in &self.alias_order {
            let (pattern, replacements) = &self.tsconfig.paths[*idx];
            if let Some(capture) = match_alias(pattern, specifier) {
                for replacement in replacements {
                    let resolved = replacement.replace('*', &capture);
                    let base = self
                        .tsconfig
                        .base_url
                        .as_ref()
                        .unwrap_or(&self.tsconfig.paths_dir)
                        .join(&resolved);
                    if let Some(p) = self.try_path(&base) {
                        return Some(p);
                    }
                }
            }
        }

        if let Some(base_url) = &self.tsconfig.base_url {
            if let Some(p) = self.try_path(&base_url.join(specifier)) {
                return Some(p);
            }
        }

        None
    }

    /// Try `base` as-is, then with each known extension appended, then as an index file.
    fn try_path(&self, base: &Path) -> Option<PathBuf> {
        let base = normalize_path(base);
        let s = base.to_string_lossy();
        if self.path_is_file(&base) {
            return Some(base);
        }
        if has_explicit_extension(&base) {
            return None;
        }

        for ext in EXTENSIONS {
            let candidate = PathBuf::from(format!("{}{}", s, ext));
            if self.path_exists(&candidate) {
                return Some(candidate);
            }
        }

        for ext in EXTENSIONS {
            let candidate = base.join(format!("index{}", ext));
            if self.path_exists(&candidate) {
                return Some(candidate);
            }
        }

        None
    }

    fn path_exists(&self, path: &Path) -> bool {
        self.visible
            .map(|visible| visible.contains(path))
            .unwrap_or_else(|| path.exists())
    }

    fn path_is_file(&self, path: &Path) -> bool {
        self.visible
            .map(|visible| visible.contains(path))
            .unwrap_or_else(|| path.is_file())
    }
}

/// Resolve `.` and `..` components without touching the filesystem.
pub fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut parts: Vec<Component> = Vec::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(parts.last(), Some(Component::Normal(_))) {
                    parts.pop();
                } else {
                    parts.push(c);
                }
            }
            other => parts.push(other),
        }
    }
    parts.iter().collect()
}

/// Try to match `specifier` against `pattern` (which may contain a single `*`).
/// Returns `Some(capture)` where `capture` is what the `*` matched, or `""` for exact.
fn match_alias(pattern: &str, specifier: &str) -> Option<String> {
    if let Some(star) = pattern.find('*') {
        let prefix = &pattern[..star];
        let suffix = &pattern[star + 1..];
        if specifier.starts_with(prefix) && specifier.ends_with(suffix) {
            let cap_end = specifier.len() - suffix.len();
            let cap_start = prefix.len();
            return (cap_start <= cap_end).then(|| specifier[cap_start..cap_end].to_string());
        }
        None
    } else if specifier == pattern {
        Some(String::new())
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
