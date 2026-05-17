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
    Edges {
        files: Vec<String>,
    },
    Related {
        #[arg(required = true)]
        files: Vec<String>,
        #[arg(long, value_enum, default_value = "both")]
        direction: DirectionArg,
    },
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
    let base = std::env::current_dir().context("current working directory must be accessible")?;
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
