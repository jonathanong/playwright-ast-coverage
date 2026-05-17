use crate::report::{print_check, print_edges, print_related, Format};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use no_mistakes_core::cli::{init_rayon_threads, resolve_root, JobsArg};
use no_mistakes_core::queue::{analyze_project, related, RelatedDirection};
use std::path::PathBuf;
use std::process::ExitCode;

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
    /// Print queue dependency edges.
    Edges {
        /// Only show edges whose source exactly matches these files/nodes.
        files: Vec<String>,
    },
    /// Print files/nodes related to the given files/nodes.
    Related {
        /// Files or queue job nodes such as queues.ts#sendWelcome.
        #[arg(required = true)]
        files: Vec<String>,
        /// Traverse dependencies, dependents, or both directions.
        #[arg(long, value_enum, default_value = "both")]
        direction: DirectionArg,
    },
    /// Check for unmatched static producers and workers.
    Check,
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
    if cli.timings {
        eprintln!("search: 0.000ms");
    }
    let report = analyze_project(&root, cli.tsconfig.as_deref(), &cli.filters)?;
    let format = cli.format.unwrap_or({
        if cli.json {
            Format::Json
        } else {
            Format::Human
        }
    });
    match &cli.command {
        Command::Edges { files } => {
            print_edges(&report, files, cli.depth, format)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Related { files, direction } => {
            let edges = related(&report, files, (*direction).into());
            print_related(files, &edges, format)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Check => {
            print_check(&report, format)?;
            Ok(if report.check.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
    }
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
