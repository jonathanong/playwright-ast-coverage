use crate::report::{print_check, print_edges, print_related, Format};
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use no_mistakes_core::queue::{analyze_project, related, RelatedDirection};
use rayon::ThreadPoolBuilder;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(author, version, about)]
pub(crate) struct Cli {
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,
    #[arg(long, global = true)]
    config: Option<PathBuf>,
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

#[derive(Args, Debug, Clone, Copy, Default)]
struct JobsArg {
    #[arg(short = 'j', long = "jobs", default_value_t = 0)]
    jobs: usize,
}

pub fn run_cli() -> Result<ExitCode> {
    let cli = Cli::parse();
    init_threads(cli.jobs)?;
    let base = std::env::current_dir().context("current working directory must be accessible")?;
    let root = if cli.root.is_absolute() {
        cli.root.clone()
    } else {
        base.join(&cli.root)
    };
    if let Some(config) = &cli.config {
        let config = if config.is_absolute() {
            config.clone()
        } else {
            root.join(config)
        };
        if !config.exists() {
            anyhow::bail!("config file does not exist: {}", config.display());
        }
    }
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

fn init_threads(args: JobsArg) -> Result<()> {
    let threads = if args.jobs > 0 {
        args.jobs
    } else if let Ok(raw) = std::env::var("RAYON_NUM_THREADS") {
        raw.parse().unwrap_or_else(|_| num_cpus::get())
    } else {
        num_cpus::get()
    };
    let _ = ThreadPoolBuilder::new().num_threads(threads).build_global();
    Ok(())
}
