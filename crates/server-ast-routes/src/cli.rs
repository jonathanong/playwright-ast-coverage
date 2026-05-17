use crate::report::{print_edges, print_related, print_routes, Format};
use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use no_mistakes_core::cli::{init_rayon_threads, resolve_root, JobsArg};
use no_mistakes_core::server_routes::{analyze_project, related, RelatedDirection};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

#[derive(Parser)]
#[command(author, version, about)]
pub(crate) struct Cli {
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,
    #[arg(long, global = true)]
    tsconfig: Option<PathBuf>,
    #[arg(long = "filter", global = true)]
    filters: Vec<String>,
    #[arg(long, global = true)]
    depth: Option<usize>,
    #[arg(long, value_enum, global = true, conflicts_with = "json")]
    format: Option<Format>,
    #[arg(long, global = true, conflicts_with = "format")]
    json: bool,
    #[arg(long, global = true)]
    timings: bool,
    #[command(flatten)]
    jobs: JobsArg,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Routes {
        files: Vec<String>,
    },
    Edges {
        roots: Vec<String>,
    },
    Related {
        #[arg(required = true)]
        roots: Vec<String>,
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
    let base = std::env::current_dir()?;
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
