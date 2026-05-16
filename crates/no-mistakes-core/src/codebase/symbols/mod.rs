//! `symbols` binary: dump the named exports and imports of one or more TS/JS files.
//!
//! Wraps `crate::codebase::ts_symbols::extract_symbols` and renders the result as JSON
//! (default for non-TTY), YAML, Markdown, paths, or a human tree.
//!
//! Re-export `source` and import `source` specifiers are resolved through
//! `crate::codebase::ts_resolver` to project-relative paths when possible, so an agent
//! can follow the chain without a second tool invocation.

pub mod output;

use anyhow::{Context, Result};
use clap::Parser;
use is_terminal::IsTerminal;
use rayon::prelude::*;
use std::io;
use std::path::{Path, PathBuf};

pub use crate::codebase::dependencies::Format;
use crate::codebase::ts_resolver::{find_tsconfig, load_tsconfig, resolve_import, TsConfig};
use crate::codebase::ts_symbols::{extract_symbols, Export, ExportKind, FileSymbols, NamedImport};

/// Which sections of each file's symbols to emit.
#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum, Default)]
pub enum Include {
    #[default]
    Exports,
    Imports,
    Both,
}

/// `--kind` filter values, validated by clap at parse time. Maps 1:1 onto
/// `crate::codebase::ts_symbols::ExportKind` so a typo like `--kind functoin` is rejected
/// with a helpful error instead of silently producing an empty result set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, clap::ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum ExportKindArg {
    Function,
    Class,
    Const,
    Let,
    Var,
    Type,
    Interface,
    Enum,
    Default,
    ReExport,
}

impl ExportKindArg {
    /// Returns `true` iff this arg matches the given `ExportKind` extracted from a file.
    fn matches(&self, k: &ExportKind) -> bool {
        matches!(
            (self, k),
            (Self::Function, ExportKind::Function)
                | (Self::Class, ExportKind::Class)
                | (Self::Const, ExportKind::Const)
                | (Self::Let, ExportKind::Let)
                | (Self::Var, ExportKind::Var)
                | (Self::Type, ExportKind::TypeAlias)
                | (Self::Interface, ExportKind::Interface)
                | (Self::Enum, ExportKind::Enum)
                | (Self::Default, ExportKind::Default)
                | (Self::ReExport, ExportKind::ReExport { .. })
        )
    }
}

/// CLI args for the `symbols` binary.
#[derive(Parser, Debug)]
pub struct SymbolsArgs {
    /// One or more TS/JS files to inspect (relative to --root or absolute).
    #[arg(required = true, value_name = "FILE")]
    pub files: Vec<PathBuf>,

    /// Project root (default: current working directory).
    #[arg(long, value_name = "PATH")]
    pub root: Option<PathBuf>,

    /// Path to tsconfig.json for resolving re-export / import specifiers.
    /// If omitted, searches upward from --root.
    #[arg(long, value_name = "FILE")]
    pub tsconfig: Option<PathBuf>,

    /// Only include exports of this kind. Repeatable. Validated by clap.
    #[arg(long = "kind", value_enum, value_name = "KIND")]
    pub kinds: Vec<ExportKindArg>,

    /// Which sections to emit: `exports` (default), `imports`, or `both`.
    #[arg(long, value_enum, default_value_t = Include::Exports)]
    pub include: Include,

    /// Output format: json, md, yml, paths, human.
    /// Defaults to human on TTY, json on non-TTY.
    #[arg(long, value_name = "FORMAT")]
    pub format: Option<Format>,

    /// Shorthand for `--format json`.
    #[arg(long, default_value_t = false)]
    pub json: bool,

    /// Emit phase timings to stderr.
    #[arg(long, default_value_t = false)]
    pub timings: bool,
}

/// One file's extracted symbols, ready to render.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Path relative to `--root` (or absolute if outside the root).
    pub rel_path: PathBuf,
    /// Symbols, with re-export sources resolved when possible.
    pub exports: Vec<ResolvedExport>,
    pub imports: Vec<ResolvedImport>,
}

/// An export with its re-export source resolved (when applicable) to a project-relative path.
#[derive(Debug, Clone)]
pub struct ResolvedExport {
    pub name: String,
    pub kind: ExportKind,
    pub line: u32,
    /// Resolved re-export target path, relative to `--root`. `None` for non-re-exports
    /// or when the source can't be resolved (e.g. bare npm specifier).
    pub resolved: Option<PathBuf>,
}

/// An import with the source specifier resolved (when applicable) to a project-relative path.
#[derive(Debug, Clone)]
pub struct ResolvedImport {
    pub source: String,
    pub imported: String,
    pub local: String,
    pub line: u32,
    pub is_type_only: bool,
    /// Resolved target path, relative to `--root`. `None` for bare npm specifiers.
    pub resolved: Option<PathBuf>,
}

/// Resolve `--root` against cwd, returning the absolute project root.
fn resolve_root(arg: Option<&Path>, cwd: &Path) -> PathBuf {
    match arg {
        Some(p) if p.is_absolute() => p.to_path_buf(),
        Some(p) => cwd.join(p),
        None => cwd.to_path_buf(),
    }
}

/// Load tsconfig from `--tsconfig` if given, else search upward from `root`,
/// else return an empty config.
fn resolve_tsconfig(arg: Option<&Path>, root: &Path) -> Result<TsConfig> {
    if let Some(path) = arg {
        return load_tsconfig(path).with_context(|| format!("loading tsconfig {}", path.display()));
    }
    if let Some(path) = find_tsconfig(root) {
        return load_tsconfig(&path)
            .with_context(|| format!("loading tsconfig {}", path.display()));
    }
    Ok(TsConfig {
        dir: root.to_path_buf(),
        paths: vec![],
        paths_dir: root.to_path_buf(),
    })
}

/// Resolve each input file path against `--root` first, falling back to cwd.
fn resolve_input_files(files: &[PathBuf], root: &Path, cwd: &Path) -> Vec<PathBuf> {
    files
        .iter()
        .map(|f| {
            if f.is_absolute() {
                f.clone()
            } else {
                let from_root = root.join(f);
                if from_root.exists() {
                    from_root
                } else {
                    cwd.join(f)
                }
            }
        })
        .collect()
}

/// End-to-end: turn a parsed `SymbolsArgs` into per-file `FileEntry` results
/// plus the original root-string list (for output formatters).
///
/// This is the shared pipeline used by both `run()` (which then writes output
/// to stdout) and the test harness (which captures into a buffer). Keeping the
/// production path and the test path on the same code prevents silent drift.
pub fn collect_entries(args: &SymbolsArgs) -> Result<(Vec<FileEntry>, Vec<String>)> {
    collect_entries_with_timings(args, None)
}

fn collect_entries_with_timings(
    args: &SymbolsArgs,
    mut timings: Option<&mut crate::codebase::timing::PhaseTimings>,
) -> Result<(Vec<FileEntry>, Vec<String>)> {
    let cwd = std::env::current_dir()?;
    let root = resolve_root(args.root.as_deref(), &cwd);
    let tsconfig = resolve_tsconfig(args.tsconfig.as_deref(), &root)?;
    let abs_files = resolve_input_files(&args.files, &root, &cwd);
    if let Some(timings) = &mut timings {
        timings.mark("search");
    }

    let kind_filter = build_kind_filter(&args.kinds);
    if let Some(timings) = &mut timings {
        timings.mark("ingest");
    }

    let entries: Vec<FileEntry> = abs_files
        .par_iter()
        .map(|abs| build_entry(abs, &root, &tsconfig, args.include, kind_filter.as_ref()))
        .collect::<Result<Vec<_>>>()?;
    if let Some(timings) = &mut timings {
        timings.mark("parse+analysis");
    }

    let root_strs: Vec<String> = args.files.iter().map(|f| f.display().to_string()).collect();
    Ok((entries, root_strs))
}

pub fn run(args: SymbolsArgs) -> Result<()> {
    let mut timings = crate::codebase::timing::PhaseTimings::start();
    let (entries, root_strs) = collect_entries_with_timings(&args, Some(&mut timings))?;

    let format = if args.json {
        Format::Json
    } else if let Some(f) = args.format {
        f
    } else if io::stdout().is_terminal() {
        Format::Human
    } else {
        Format::Json
    };

    let stdout = io::stdout();
    let mut out = stdout.lock();
    match format {
        Format::Json => output::write_json(&root_strs, &entries, &mut out)?,
        Format::Md => output::write_md(&root_strs, &entries, &mut out)?,
        Format::Yml => output::write_yml(&root_strs, &entries, &mut out)?,
        Format::Paths => output::write_paths(&entries, &mut out)?,
        Format::Human => output::write_human(&root_strs, &entries, &mut out)?,
    }
    timings.mark("output");
    if args.timings {
        timings.print_stderr();
    }
    Ok(())
}

fn build_entry(
    abs_path: &Path,
    root: &Path,
    tsconfig: &TsConfig,
    include: Include,
    kind_filter: Option<&KindFilter>,
) -> Result<FileEntry> {
    let source = std::fs::read_to_string(abs_path)
        .with_context(|| format!("reading {}", abs_path.display()))?;
    let is_tsx = matches!(
        abs_path.extension().and_then(|s| s.to_str()),
        Some("tsx") | Some("jsx")
    );
    let symbols: FileSymbols = extract_symbols(&source, is_tsx)
        .with_context(|| format!("parsing {}", abs_path.display()))?;

    let want_exports = matches!(include, Include::Exports | Include::Both);
    let want_imports = matches!(include, Include::Imports | Include::Both);

    let exports = if want_exports {
        symbols
            .exports
            .into_iter()
            .filter(|e| match kind_filter {
                Some(kf) => kf.matches_export(&e.kind),
                None => true,
            })
            .map(|e| resolve_export(e, abs_path, root, tsconfig))
            .collect()
    } else {
        Vec::new()
    };

    let imports = if want_imports {
        symbols
            .imports
            .into_iter()
            .map(|i| resolve_named_import(i, abs_path, root, tsconfig))
            .collect()
    } else {
        Vec::new()
    };

    let rel_path = make_relative(abs_path, root);

    Ok(FileEntry {
        rel_path,
        exports,
        imports,
    })
}

fn resolve_export(e: Export, abs_path: &Path, root: &Path, tsconfig: &TsConfig) -> ResolvedExport {
    let resolved = if let ExportKind::ReExport { source, .. } = &e.kind {
        resolve_import(source, abs_path, tsconfig).map(|abs| make_relative(&abs, root))
    } else {
        None
    };
    ResolvedExport {
        name: e.name,
        kind: e.kind,
        line: e.line,
        resolved,
    }
}

fn resolve_named_import(
    i: NamedImport,
    abs_path: &Path,
    root: &Path,
    tsconfig: &TsConfig,
) -> ResolvedImport {
    let resolved =
        resolve_import(&i.source, abs_path, tsconfig).map(|abs| make_relative(&abs, root));
    ResolvedImport {
        source: i.source,
        imported: i.imported,
        local: i.local,
        line: i.line,
        is_type_only: i.is_type_only,
        resolved,
    }
}

fn make_relative(abs: &Path, root: &Path) -> PathBuf {
    abs.strip_prefix(root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| abs.to_path_buf())
}

/// Parsed `--kind` filter set, or `None` if no filter was given.
pub(crate) struct KindFilter {
    allowed: std::collections::HashSet<ExportKindArg>,
}

impl KindFilter {
    fn matches_export(&self, k: &ExportKind) -> bool {
        self.allowed.iter().any(|arg| arg.matches(k))
    }
}

fn build_kind_filter(kinds: &[ExportKindArg]) -> Option<KindFilter> {
    if kinds.is_empty() {
        return None;
    }
    Some(KindFilter {
        allowed: kinds.iter().copied().collect(),
    })
}

/// Stable string name for an `ExportKind` — used as the `kind` field in JSON output.
pub fn export_kind_str(k: &ExportKind) -> &'static str {
    match k {
        ExportKind::Function => "function",
        ExportKind::Class => "class",
        ExportKind::Const => "const",
        ExportKind::Let => "let",
        ExportKind::Var => "var",
        ExportKind::TypeAlias => "type",
        ExportKind::Interface => "interface",
        ExportKind::Enum => "enum",
        ExportKind::Default => "default",
        ExportKind::ReExport { .. } => "re-export",
    }
}

#[cfg(test)]
mod tests;
