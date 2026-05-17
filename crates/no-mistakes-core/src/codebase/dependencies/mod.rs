pub mod extract;
pub mod graph;
pub mod output;

use anyhow::{bail, Context, Result};
use is_terminal::IsTerminal;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

pub use crate::codebase::ts_resolver::TsConfig;
pub use graph::{DepGraph, EdgeKind, NodeId};

pub use crate::cli::Format;

/// Map a `--test <framework>` value to its corresponding glob patterns.
pub(crate) fn test_globs(framework: &str) -> Vec<String> {
    match framework {
        "vitest" => vec![
            "**/*.test.mts".to_string(),
            "**/*.spec.mts".to_string(),
            "**/*.test.ts".to_string(),
            "**/*.spec.ts".to_string(),
            "**/*.test.tsx".to_string(),
            "**/*.spec.tsx".to_string(),
            "**/*.test.mjs".to_string(),
            "**/*.spec.mjs".to_string(),
            "**/*.test.js".to_string(),
            "**/*.spec.js".to_string(),
            "**/*.test.jsx".to_string(),
            "**/*.spec.jsx".to_string(),
        ],
        "playwright" => vec![
            "**/tests/e2e/**/*.mts".to_string(),
            "**/tests/e2e/**/*.ts".to_string(),
            "**/tests/e2e/**/*.tsx".to_string(),
            "**/tests/e2e/**/*.mjs".to_string(),
            "**/tests/e2e/**/*.js".to_string(),
            "**/tests/e2e/**/*.jsx".to_string(),
            "**/playwright/**/*.spec.mts".to_string(),
            "**/playwright/**/*.spec.ts".to_string(),
            "**/playwright/**/*.spec.tsx".to_string(),
            "**/playwright/**/*.spec.mjs".to_string(),
            "**/playwright/**/*.spec.js".to_string(),
            "**/playwright/**/*.spec.jsx".to_string(),
        ],
        "cargo" => vec![
            "**/tests/**/*.rs".to_string(),
            "src/**/*_test.rs".to_string(),
        ],
        _ => vec![],
    }
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
fn relationship_filter(
    relationships: &[RelationshipArg],
) -> Option<std::collections::HashSet<EdgeKind>> {
    if relationships.is_empty() || relationships.contains(&RelationshipArg::All) {
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
            RelationshipArg::All => {}
        }
    }
    if set.is_empty() {
        None
    } else {
        Some(set)
    }
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
    let cwd_early = std::env::current_dir()?;
    let root = match args.root {
        Some(p) => {
            if p.is_absolute() {
                p
            } else {
                cwd_early.join(p)
            }
        }
        None => cwd_early,
    };
    let root = crate::codebase::ts_resolver::normalize_path(&root);

    let tsconfig = match args.tsconfig {
        Some(ref path) => crate::codebase::ts_resolver::load_tsconfig(path)
            .with_context(|| format!("loading tsconfig {}", path.display()))?,
        None => match crate::codebase::ts_resolver::find_tsconfig(&root) {
            Some(path) => crate::codebase::ts_resolver::load_tsconfig(&path)
                .with_context(|| format!("loading tsconfig {}", path.display()))?,
            None => crate::codebase::ts_resolver::TsConfig {
                dir: root.clone(),
                paths: vec![],
                paths_dir: root.clone(),
            },
        },
    };

    let cwd = std::env::current_dir().unwrap_or_else(|_| root.clone());

    // Parse entrypoints, resolving to absolute paths.
    // Relative paths are tried against --root first, then cwd as fallback.
    let entrypoints: Vec<Entrypoint> = args
        .files
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
        .collect();

    let root_strs: Vec<String> = args.files.iter().map(|f| f.display().to_string()).collect();

    timings.mark("search");

    // Check for #symbol used in Deps direction (unsupported).
    if matches!(direction, Direction::Deps) {
        for ep in &entrypoints {
            if ep.symbol.is_some() {
                bail!(
                    "#symbol targeting (e.g. `file.mts#exportName`) is only supported \
                     in the `dependents` direction. For `dependencies`, use a plain file path."
                );
            }
        }
    }

    let allowed = relationship_filter(&args.relationships);
    let build_plan = graph::GraphBuildPlan::from_allowed(allowed.as_ref());
    let graph_files = graph::GraphFiles::discover(&root);

    timings.mark("ingest");

    let entries = match direction {
        Direction::Deps => {
            let roots: Vec<NodeId> = entrypoints
                .iter()
                .map(|e| NodeId::File(e.file.clone()))
                .collect();
            if relationships_are_import_only(&args.relationships) {
                graph::lazy_import_deps_of_with_files(
                    &roots,
                    &root,
                    &tsconfig,
                    args.depth,
                    &graph_files,
                )
                .context("walking import dependencies lazily")?
            } else {
                let graph = graph::DepGraph::build_with_plan_and_files(
                    &root,
                    &tsconfig,
                    build_plan,
                    &graph_files,
                )
                .context("building dependency graph")?;
                graph.deps_of(&roots, args.depth, allowed.as_ref())
            }
        }
        Direction::Dependents => {
            let any_symbol = entrypoints.iter().any(|e| e.symbol.is_some());
            let symbol_facts = any_symbol.then(|| {
                crate::codebase::ts_source::facts::collect_ts_facts(
                    graph_files.indexable(),
                    crate::codebase::ts_source::facts::TsFactPlan::imports_and_symbols(),
                )
            });
            let graph = match symbol_facts.as_ref() {
                Some(facts) => graph::DepGraph::build_with_plan_files_and_facts(
                    &root,
                    &tsconfig,
                    build_plan,
                    &graph_files,
                    Some(facts),
                ),
                None => graph::DepGraph::build_with_plan_and_files(
                    &root,
                    &tsconfig,
                    build_plan,
                    &graph_files,
                ),
            }
            .context("building dependency graph")?;
            if any_symbol {
                let mut all_entries: HashMap<NodeId, graph::NodeEntry> = HashMap::new();
                let symbol_index = graph::SymbolIndex::build_from_facts(
                    &tsconfig,
                    &graph_files,
                    symbol_facts
                        .as_ref()
                        .expect("symbol facts are collected for symbol queries"),
                )?;
                for ep in &entrypoints {
                    if let Some(sym) = &ep.symbol {
                        let entries = graph.dependents_of_symbol(
                            &ep.file,
                            sym,
                            args.depth,
                            allowed.as_ref(),
                            &symbol_index,
                        );
                        merge_node_entries(&mut all_entries, entries);
                    } else {
                        let entries = graph.dependents_of(
                            std::slice::from_ref(&NodeId::File(ep.file.clone())),
                            args.depth,
                            allowed.as_ref(),
                        );
                        merge_node_entries(&mut all_entries, entries);
                    }
                }
                let mut entries: Vec<_> = all_entries.into_values().collect();
                entries.sort_by(|a, b| {
                    a.depth
                        .cmp(&b.depth)
                        .then_with(|| a.node.display_name(&root).cmp(&b.node.display_name(&root)))
                });
                entries
            } else {
                let roots: Vec<NodeId> = entrypoints
                    .iter()
                    .map(|e| NodeId::File(e.file.clone()))
                    .collect();
                graph.dependents_of(&roots, args.depth, allowed.as_ref())
            }
        }
    };

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
        Format::Json => output::write_json(&root_strs, &entries, &root, &mut out)?,
        Format::Md => output::write_md(&root_strs, &entries, &root, &mut out)?,
        Format::Yml => output::write_yml(&root_strs, &entries, &root, &mut out)?,
        Format::Paths => output::write_paths(&entries, &root, &mut out)?,
        Format::Human => output::write_human(&root_strs, &entries, &root, &mut out)?,
    }

    timings.mark("output");
    if args.timings {
        timings.print_stderr();
    }

    Ok(())
}

fn merge_node_entries(
    merged: &mut HashMap<NodeId, graph::NodeEntry>,
    entries: Vec<graph::NodeEntry>,
) {
    for entry in entries {
        merged
            .entry(entry.node.clone())
            .and_modify(|existing| {
                existing.depth = existing.depth.min(entry.depth);
                existing.via.extend(entry.via.iter().copied());
                existing.via.sort_by_key(|kind| *kind as u8);
                existing.via.dedup();
            })
            .or_insert(entry);
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
