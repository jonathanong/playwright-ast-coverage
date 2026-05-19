pub mod extract;
pub mod graph;
pub mod output;

use anyhow::{bail, Context, Result};
use is_terminal::IsTerminal;
use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};

pub use crate::codebase::ts_resolver::TsConfig;
pub use graph::{DepGraph, EdgeKind, NodeId};

pub use crate::cli::Format;

pub(crate) const VITEST_JEST_TEST_GLOBS: &[&str] = &[
    "**/*.test.mts",
    "**/*.spec.mts",
    "**/*.test.ts",
    "**/*.spec.ts",
    "**/*.test.tsx",
    "**/*.spec.tsx",
    "**/*.test.mjs",
    "**/*.spec.mjs",
    "**/*.test.js",
    "**/*.spec.js",
    "**/*.test.jsx",
    "**/*.spec.jsx",
    "**/__tests__/**/*.mts",
    "**/__tests__/**/*.ts",
    "**/__tests__/**/*.tsx",
    "**/__tests__/**/*.mjs",
    "**/__tests__/**/*.js",
    "**/__tests__/**/*.jsx",
];

/// Map a `--test <framework>` value to its corresponding glob patterns.
pub(crate) fn test_globs(framework: &str) -> Vec<String> {
    const PLAYWRIGHT: &[&str] = &[
        "**/tests/e2e/**/*.mts",
        "**/tests/e2e/**/*.ts",
        "**/tests/e2e/**/*.tsx",
        "**/tests/e2e/**/*.mjs",
        "**/tests/e2e/**/*.js",
        "**/tests/e2e/**/*.jsx",
        "**/playwright/**/*.spec.mts",
        "**/playwright/**/*.spec.ts",
        "**/playwright/**/*.spec.tsx",
        "**/playwright/**/*.spec.mjs",
        "**/playwright/**/*.spec.js",
        "**/playwright/**/*.spec.jsx",
    ];
    const CARGO: &[&str] = &["**/tests/**/*.rs", "src/**/*_test.rs"];

    match framework {
        "vitest" => globs_to_strings(VITEST_JEST_TEST_GLOBS),
        "jest" => globs_to_strings(VITEST_JEST_TEST_GLOBS),
        "playwright" => globs_to_strings(PLAYWRIGHT),
        "cargo" => globs_to_strings(CARGO),
        _ => vec![],
    }
}

fn globs_to_strings(globs: &[&str]) -> Vec<String> {
    let mut strings = Vec::with_capacity(globs.len());
    for glob in globs {
        strings.push((*glob).to_string());
    }
    strings
}

pub enum Direction {
    Deps,
    Dependents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum RelationshipArg {
    Import,
    Workspace,
    Test,
    Route,
    Queue,
    Md,
    Ci,
    Http,
    Process,
    All,
}

/// Convert `--relationship` values into a `HashSet<EdgeKind>` filter.
/// Returns `None` when "all" is present or the list is empty (= no filter).
#[inline(never)]
fn relationship_filter(
    relationships: &[RelationshipArg],
) -> Option<std::collections::HashSet<EdgeKind>> {
    if relationships.is_empty() {
        return None;
    }
    let mut set = std::collections::HashSet::new();
    for r in relationships {
        match r {
            RelationshipArg::Import => {
                set.insert(EdgeKind::Import);
                set.insert(EdgeKind::TypeImport);
                set.insert(EdgeKind::DynamicImport);
                set.insert(EdgeKind::Require);
            }
            RelationshipArg::Workspace => {
                set.insert(EdgeKind::WorkspaceImport);
            }
            RelationshipArg::Test => {
                set.insert(EdgeKind::TestOf);
                set.insert(EdgeKind::RouteTest);
            }
            RelationshipArg::Route => {
                set.insert(EdgeKind::RouteRef);
                set.insert(EdgeKind::RouteTest);
            }
            RelationshipArg::Queue => {
                set.insert(EdgeKind::QueueEnqueue);
                set.insert(EdgeKind::QueueWorker);
            }
            RelationshipArg::Md => {
                set.insert(EdgeKind::MarkdownLink);
            }
            RelationshipArg::Ci => {
                set.insert(EdgeKind::CiInvocation);
            }
            RelationshipArg::Http => {
                set.insert(EdgeKind::HttpCall);
            }
            RelationshipArg::Process => {
                set.insert(EdgeKind::ProcessSpawn);
            }
            RelationshipArg::All => return None,
        }
    }
    Some(set)
}

fn relationships_are_import_only(relationships: &[RelationshipArg]) -> bool {
    !relationships.is_empty()
        && relationships
            .iter()
            .all(|relationship| *relationship == RelationshipArg::Import)
}

/// A parsed entrypoint: either a plain file path, or a file + exported symbol / queue job name.
struct Entrypoint {
    file: PathBuf,
    symbol: Option<String>,
}

fn parse_entrypoint(s: &str) -> Entrypoint {
    match s.split_once('#') {
        Some((file, symbol)) => Entrypoint {
            file: PathBuf::from(file),
            symbol: Some(symbol.to_string()),
        },
        None => Entrypoint {
            file: PathBuf::from(s),
            symbol: None,
        },
    }
}

pub fn run(args: TraverseArgs, direction: Direction) -> Result<()> {
    let mut timings = crate::codebase::timing::PhaseTimings::start();
    let cwd_early = std::env::current_dir().context("reading current directory")?;
    let root = resolve_root(&args, &cwd_early);
    let root = crate::codebase::ts_resolver::normalize_path(&root);

    let tsconfig = resolve_tsconfig(&args, &root)?;
    let entrypoints = resolve_entrypoints(&args.files, &root, &cwd_early);

    let root_strs: Vec<String> = args.files.iter().map(|f| f.display().to_string()).collect();

    timings.mark("search");

    // Check for #symbol used in Deps direction (unsupported).
    validate_direction(&direction, &entrypoints)?;

    let allowed = relationship_filter(&args.relationships);
    let build_plan = graph::GraphBuildPlan::from_allowed(allowed.as_ref());
    let graph_files = graph::GraphFiles::discover(&root);
    let ctx = TraversalCtx {
        root: &root,
        tsconfig: &tsconfig,
        graph_files: &graph_files,
        build_plan,
        allowed: allowed.as_ref(),
    };
    let roots: Vec<NodeId> = entrypoints
        .iter()
        .map(|e| NodeId::File(e.file.clone()))
        .collect();
    let import_only = relationships_are_import_only(&args.relationships);

    timings.mark("ingest");

    let entries = get_entries(
        direction,
        &roots,
        &entrypoints,
        args.depth,
        import_only,
        &ctx,
    );

    timings.mark("parse");

    // Build combined filter from --filter and --test globs.
    let mut all_filters = args.filters.clone();
    for framework in &args.tests {
        all_filters.extend(test_globs(framework));
    }
    let filter = graph::build_filter(&all_filters)?;
    let entries = graph::apply_filter(entries, filter.as_ref(), &root);

    timings.mark("analysis");

    // Resolve output format.
    let format = resolve_format(args.json, args.format, io::stdout().is_terminal());

    let stdout = io::stdout();
    let mut out = stdout.lock();

    write_entries(format, &root_strs, &entries, &root, &mut out)?;

    timings.mark("output");
    if args.timings {
        timings.print_stderr();
    }

    Ok(())
}

struct TraversalCtx<'a> {
    root: &'a Path,
    tsconfig: &'a TsConfig,
    graph_files: &'a graph::GraphFiles,
    build_plan: graph::GraphBuildPlan,
    allowed: Option<&'a std::collections::HashSet<EdgeKind>>,
}

fn resolve_tsconfig(args: &TraverseArgs, root: &Path) -> Result<TsConfig> {
    match args.tsconfig {
        Some(ref path) => crate::codebase::ts_resolver::load_tsconfig(path),
        None => match crate::codebase::ts_resolver::find_tsconfig(root) {
            Some(path) => crate::codebase::ts_resolver::load_tsconfig(&path),
            None => Ok(crate::codebase::ts_resolver::TsConfig {
                dir: root.to_path_buf(),
                paths: vec![],
                paths_dir: root.to_path_buf(),
                base_url: None,
            }),
        },
    }
}

fn resolve_root(args: &TraverseArgs, cwd: &Path) -> PathBuf {
    match &args.root {
        Some(p) => {
            if p.is_absolute() {
                p.clone()
            } else {
                cwd.join(p)
            }
        }
        None => cwd.to_path_buf(),
    }
}

fn resolve_entrypoints(raw_entrypoints: &[PathBuf], root: &Path, cwd: &Path) -> Vec<Entrypoint> {
    raw_entrypoints
        .iter()
        .map(|raw| {
            let raw_str = raw.to_string_lossy();
            let ep = parse_entrypoint(&raw_str);
            let file = if ep.file.is_absolute() {
                ep.file
            } else {
                let from_root = root.join(&ep.file);
                if from_root.exists() {
                    from_root
                } else {
                    cwd.join(ep.file)
                }
            };
            Entrypoint {
                file,
                symbol: ep.symbol,
            }
        })
        .collect()
}

fn validate_direction(direction: &Direction, entrypoints: &[Entrypoint]) -> Result<()> {
    if matches!(direction, Direction::Deps) {
        for ep in entrypoints {
            if ep.symbol.is_some() {
                bail!(
                    "#symbol targeting (e.g. `file.mts#exportName`) is only supported \
                     in the `dependents` direction. For `dependencies`, use a plain file path."
                );
            }
        }
    }
    Ok(())
}

fn deps_entries(
    depth: Option<usize>,
    import_only: bool,
    roots: &[NodeId],
    ctx: &TraversalCtx<'_>,
) -> Vec<graph::NodeEntry> {
    if import_only {
        graph::lazy_import_deps_of_with_files(roots, ctx.root, ctx.tsconfig, depth, ctx.graph_files)
    } else {
        graph::DepGraph::build_with_plan_and_files(
            ctx.root,
            ctx.tsconfig,
            ctx.build_plan,
            ctx.graph_files,
        )
        .deps_of(roots, depth, ctx.allowed)
    }
}

fn get_entries(
    direction: Direction,
    roots: &[NodeId],
    entrypoints: &[Entrypoint],
    depth: Option<usize>,
    import_only: bool,
    ctx: &TraversalCtx<'_>,
) -> Vec<graph::NodeEntry> {
    match direction {
        Direction::Deps => deps_entries(depth, import_only, roots, ctx),
        Direction::Dependents => dependents_entries(entrypoints, roots, depth, ctx),
    }
}

fn dependents_entries(
    entrypoints: &[Entrypoint],
    roots: &[NodeId],
    depth: Option<usize>,
    ctx: &TraversalCtx<'_>,
) -> Vec<graph::NodeEntry> {
    let any_symbol = entrypoints.iter().any(|e| e.symbol.is_some());
    let symbol_facts = any_symbol.then(|| {
        crate::codebase::ts_source::facts::collect_ts_facts(
            ctx.graph_files.indexable(),
            crate::codebase::ts_source::facts::TsFactPlan::imports_and_symbols(),
        )
    });
    let graph = build_dependents_graph(ctx, symbol_facts.as_ref());
    if any_symbol {
        let facts = symbol_facts
            .as_ref()
            .expect("symbol facts are collected for symbol queries");
        let symbol_index =
            graph::SymbolIndex::build_from_facts(ctx.tsconfig, ctx.graph_files, facts);
        resolve_symbol_dependents(
            ctx.root,
            entrypoints,
            depth,
            ctx.allowed,
            &graph,
            &symbol_index,
        )
    } else {
        graph.dependents_of(roots, depth, ctx.allowed)
    }
}

fn resolve_symbol_dependents(
    root: &Path,
    entrypoints: &[Entrypoint],
    depth: Option<usize>,
    allowed: Option<&std::collections::HashSet<EdgeKind>>,
    graph: &graph::DepGraph,
    symbol_index: &graph::SymbolIndex,
) -> Vec<graph::NodeEntry> {
    let mut all_entries: HashMap<NodeId, graph::NodeEntry> = HashMap::new();
    let plain_roots: Vec<_> = entrypoints
        .iter()
        .filter(|ep| ep.symbol.is_none())
        .map(|ep| NodeId::File(ep.file.clone()))
        .collect();
    if !plain_roots.is_empty() {
        let entries = graph.dependents_of(&plain_roots, depth, allowed);
        merge_node_entries(&mut all_entries, entries);
    }
    for ep in entrypoints {
        if let Some(sym) = &ep.symbol {
            let entries = graph.dependents_of_symbol(&ep.file, sym, depth, allowed, symbol_index);
            merge_node_entries(&mut all_entries, entries);
        }
    }
    let mut entries: Vec<_> = all_entries.into_values().collect();
    sort_node_entries(&mut entries, root);
    entries
}

fn build_dependents_graph(
    ctx: &TraversalCtx<'_>,
    symbol_facts: Option<&crate::codebase::ts_source::facts::TsFactMap>,
) -> graph::DepGraph {
    match symbol_facts {
        Some(facts) => graph::DepGraph::build_with_plan_files_and_facts(
            ctx.root,
            ctx.tsconfig,
            ctx.build_plan,
            ctx.graph_files,
            Some(facts),
        ),
        None => graph::DepGraph::build_with_plan_and_files(
            ctx.root,
            ctx.tsconfig,
            ctx.build_plan,
            ctx.graph_files,
        ),
    }
}

fn write_entries(
    format: Format,
    root_strs: &[String],
    entries: &[graph::NodeEntry],
    root: &Path,
    out: &mut dyn Write,
) -> Result<()> {
    match format {
        Format::Json => output::write_json(root_strs, entries, root, out),
        Format::Md => output::write_md(root_strs, entries, root, out),
        Format::Yml => output::write_yml(root_strs, entries, root, out),
        Format::Paths => output::write_paths(entries, root, out),
        Format::Human => output::write_human(root_strs, entries, root, out),
    }
}

fn resolve_format(json: bool, format: Option<Format>, stdout_is_terminal: bool) -> Format {
    if json {
        Format::Json
    } else if let Some(format) = format {
        format
    } else if stdout_is_terminal {
        Format::Human
    } else {
        Format::Json
    }
}

fn sort_node_entries(entries: &mut [graph::NodeEntry], root: &Path) {
    entries.sort_by_key(|entry| (entry.depth, entry.node.display_name(root)));
}

fn merge_node_entries(
    merged: &mut HashMap<NodeId, graph::NodeEntry>,
    entries: Vec<graph::NodeEntry>,
) {
    for entry in entries {
        if let Some(existing) = merged.get_mut(&entry.node) {
            existing.depth = existing.depth.min(entry.depth);
            existing.via.extend(entry.via.iter().copied());
            existing.via.sort_by_key(|kind| *kind as u8);
            existing.via.dedup();
        } else {
            merged.insert(entry.node.clone(), entry);
        }
    }
}

#[derive(clap::Parser)]
pub struct TraverseArgs {
    /// Files to start from. Supports `FILE#SYMBOL` for symbol-level dependents queries
    /// and `QUEUE_FILE#JOB_NAME` for queue-job dependents queries.
    /// Can be relative to --root or absolute.
    #[arg(required = true, value_name = "FILE")]
    pub files: Vec<PathBuf>,

    /// Project root directory (default: current working directory).
    #[arg(long, value_name = "PATH")]
    pub root: Option<PathBuf>,

    /// Path to tsconfig.json for path alias resolution.
    /// If omitted, searches upward from root for tsconfig.json.
    #[arg(long, value_name = "FILE")]
    pub tsconfig: Option<PathBuf>,

    /// Maximum traversal depth (default: unlimited). Alias: `--max-depth`.
    #[arg(long, alias = "max-depth", value_name = "N")]
    pub depth: Option<usize>,

    /// Only include files matching this glob pattern. Can be repeated (OR logic).
    /// Patterns ending in `/` collapse results to that folder level.
    #[arg(long = "filter", value_name = "GLOB")]
    pub filters: Vec<String>,

    /// Filter to test files for a specific framework. Can be repeated.
    /// Values: vitest, playwright, cargo.
    #[arg(long = "test", value_name = "FRAMEWORK")]
    pub tests: Vec<String>,

    /// Output format: json, md, yml, paths, human.
    /// Defaults to human on TTY, json on non-TTY.
    #[arg(long, value_name = "FORMAT", conflicts_with = "json")]
    pub format: Option<Format>,

    /// Shorthand for `--format json`.
    #[arg(long, default_value_t = false, conflicts_with = "format")]
    pub json: bool,

    /// Only follow edges of this relationship kind. Can be repeated (OR logic).
    /// Values: import, workspace, test, route, queue, md, ci, http, process, all.
    /// Default: all.
    #[arg(long = "relationship", value_enum, value_name = "KIND")]
    pub relationships: Vec<RelationshipArg>,

    /// Emit phase timings to stderr.
    #[arg(long, default_value_t = false)]
    pub timings: bool,
}

#[cfg(test)]
mod tests;
