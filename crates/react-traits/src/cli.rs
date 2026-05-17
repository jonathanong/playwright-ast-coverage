use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use no_mistakes_core::react_traits;
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
        let raw_args = std::env::var("REACT_TRAITS_TEST_ARGS")
            .expect("REACT_TRAITS_TEST_ARGS must be set in tests — use with_run_args_env()");
        Cli::parse_from(raw_args.split('\u{1f}'))
    } else {
        Cli::parse()
    };
    let base_root =
        std::env::current_dir().context("current working directory must be accessible")?;
    let root = base_root.join(&cli.root);
    match &cli.command {
        Command::Analyze { targets } => {
            let results =
                react_traits::run_analyze(&root, cli.config.as_deref(), targets, None)?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&results)
                        .expect("serialization of Rust structs never fails")
                );
            } else {
                no_mistakes_core::react_traits::report::text::print_results(&results, 0);
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Check {
            targets,
            assert_no_fetch,
        } => {
            let violations =
                react_traits::run_check(&root, cli.config.as_deref(), targets, *assert_no_fetch)?;
            if violations.is_empty() {
                Ok(ExitCode::SUCCESS)
            } else {
                if cli.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&violations)
                            .expect("serialization of Rust structs never fails")
                        );
                } else {
                    no_mistakes_core::react_traits::report::text::print_violations(&violations);
                }
                Ok(ExitCode::from(1))
            }
        }
    }
}
