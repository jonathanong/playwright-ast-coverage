use crate::pipeline::run::run_with_base_root;
use crate::report::print::print_markdown_report;
use anyhow::Result;
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

#[cfg(test)]
fn parse_cli_args() -> Cli {
    if let Ok(raw_args) = std::env::var("NEXT_TO_FETCH_TEST_ARGS") {
        Cli::parse_from(raw_args.split('\u{1f}'))
    } else {
        Cli::parse()
    }
}

#[cfg(not(test))]
fn parse_cli_args() -> Cli {
    Cli::parse()
}

pub fn run_cli() -> Result<ExitCode> {
    let cli = parse_cli_args();
    let base_root = std::env::current_dir().expect("current directory is available");
    let report = run_with_base_root(&base_root, &cli)?;
    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .expect("serialization of Rust structs never fails")
        );
    } else {
        print_markdown_report(&report);
    }
    Ok(ExitCode::SUCCESS)
}
