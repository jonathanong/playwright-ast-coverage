use crate::report::{print_edges, print_related, print_routes, Format};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use no_mistakes_core::cli::{init_rayon_threads, resolve_root, JobsArg};
use no_mistakes_core::server_routes::{analyze_project, related, RelatedDirection};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

#[derive(Parser)]
#[command(author, version, about)]
pub(crate) struct Cli {
    /// Project root directory.
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,
    /// Path to tsconfig.json for path alias resolution.
    #[arg(long, global = true)]
    tsconfig: Option<PathBuf>,
    /// Filter to source files matching this glob. Can be repeated.
    #[arg(long = "filter", global = true)]
    filters: Vec<String>,
    /// Maximum edge traversal depth for the edges command when roots are provided.
    /// Defaults to 1 when roots are provided, and unlimited otherwise.
    #[arg(long, alias = "max-depth", global = true)]
    depth: Option<usize>,
    /// Output format.
    #[arg(long, value_enum, global = true, conflicts_with = "json")]
    format: Option<Format>,
    /// Shorthand for --format json.
    #[arg(long, global = true, conflicts_with = "format")]
    json: bool,
    /// Emit phase timings to stderr.
    #[arg(long, global = true)]
    timings: bool,
    #[command(flatten)]
    jobs: JobsArg,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List extracted server routes.
    Routes {
        /// Only show routes whose file or route exactly matches one of these values.
        files: Vec<String>,
    },
    /// Print server route dependency edges.
    Edges {
        /// Only show edges whose source exactly matches these files/nodes.
        roots: Vec<String>,
    },
    /// Print files related to the given files through server route edges.
    Related {
        /// Files or route nodes to traverse from.
        #[arg(required = true)]
        roots: Vec<String>,
        /// Traverse dependencies, dependents, or both directions.
        #[arg(long, value_enum, default_value = "both")]
        direction: DirectionArg,
    },
}

#[derive(ValueEnum, Clone, Copy)]
enum DirectionArg {
    Deps,
    Dependents,
    Both,
}

pub fn run_cli() -> Result<ExitCode> {
    let cli = Cli::parse();
    init_rayon_threads(cli.jobs);
    let base = std::env::current_dir().context("reading current directory")?;
    let root = resolve_root(&cli.root, &base);
    let started = Instant::now();
    let report = analyze_project(&root, cli.tsconfig.as_deref(), &cli.filters)?;
    if cli.timings {
        eprintln!(
            "analysis: {:.3}ms",
            started.elapsed().as_secs_f64() * 1000.0
        );
    }
    let format = cli.format.unwrap_or(if cli.json {
        Format::Json
    } else {
        Format::Human
    });
    match &cli.command {
        Command::Routes { files } => print_routes(&report, files, format)?,
        Command::Edges { roots } => print_edges(&report, roots, cli.depth, format)?,
        Command::Related { roots, direction } => {
            let edges = related(&report, roots, (*direction).into());
            print_related(roots, &edges, format)?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

impl From<DirectionArg> for RelatedDirection {
    fn from(value: DirectionArg) -> Self {
        match value {
            DirectionArg::Deps => RelatedDirection::Deps,
            DirectionArg::Dependents => RelatedDirection::Dependents,
            DirectionArg::Both => RelatedDirection::Both,
        }
    }
}
