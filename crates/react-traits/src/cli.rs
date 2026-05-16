use crate::pipeline::check::run_check;
use crate::pipeline::run::run_analyze;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(author, version, about)]
pub(crate) struct Cli {
    #[arg(long, default_value = ".", global = true)]
    pub(crate) root: PathBuf,
    #[arg(long, global = true)]
    pub(crate) config: Option<PathBuf>,
    #[arg(long, global = true)]
    pub(crate) json: bool,
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    Analyze {
        #[arg(help = "Glob patterns for component files")]
        targets: Vec<String>,
        #[arg(
            long,
            help = "Max depth of child detail to print (0 = aggregated only)"
        )]
        return_depth: Option<usize>,
    },
    Check {
        #[arg(help = "Glob patterns for component files")]
        targets: Vec<String>,
        #[arg(long)]
        assert_no_fetch: bool,
    },
}

pub fn run_cli() -> Result<ExitCode> {
    let cli = if cfg!(test) {
        if let Ok(raw_args) = std::env::var("REACT_TRAITS_TEST_ARGS") {
            Cli::parse_from(raw_args.split('\u{1f}'))
        } else {
            Cli::parse()
        }
    } else {
        Cli::parse()
    };
    let base_root = std::env::current_dir()?;
    match &cli.command {
        Command::Analyze {
            targets,
            return_depth,
        } => {
            let results = run_analyze(&base_root, &cli, targets, *return_depth)?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&results)?);
            } else {
                crate::report::text::print_results(&results, return_depth.unwrap_or(0));
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Check {
            targets,
            assert_no_fetch,
        } => {
            let violations = run_check(&base_root, &cli, targets, *assert_no_fetch)?;
            if violations.is_empty() {
                Ok(ExitCode::SUCCESS)
            } else {
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&violations)?);
                } else {
                    crate::report::text::print_violations(&violations);
                }
                Ok(ExitCode::from(1))
            }
        }
    }
}
