use crate::pipeline::run::run_with_base_root;
use crate::report::print::print_markdown_report;
use anyhow::{Context, Result};
use clap::Parser;
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

    #[arg(help = "Specific routes or files to analyze")]
    pub(crate) targets: Vec<String>,
}

pub fn run_cli() -> Result<ExitCode> {
    let cli = if cfg!(test) {
        if let Ok(raw_args) = std::env::var("NEXT_TO_FETCH_TEST_ARGS") {
            Cli::parse_from(raw_args.split('\u{1f}'))
        } else {
            Cli::parse()
        }
    } else {
        Cli::parse()
    };
    let base_root =
        std::env::current_dir().context("current working directory must be accessible")?;
    let report = run_with_base_root(&base_root, &cli)?;
    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).context("failed to serialize report as JSON")?
        );
    } else {
        print_markdown_report(&report);
    }
    Ok(ExitCode::SUCCESS)
}
