use crate::pipeline::run::run_with_base_root;
use crate::report::print::print_markdown_report;
use anyhow::{Context, Result};
use clap::Parser;
use no_mistakes_core::cli::Format;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(author, version, about)]
pub(crate) struct Cli {
    /// Project root directory.
    #[arg(long, default_value = ".", global = true)]
    pub(crate) root: PathBuf,

    /// Config file path. Relative paths are resolved from --root.
    #[arg(long, global = true)]
    pub(crate) config: Option<PathBuf>,

    /// Output format: json, yml, paths, md, human.
    #[arg(
        long,
        value_enum,
        default_value = "human",
        global = true,
        conflicts_with = "json"
    )]
    pub(crate) format: Format,

    /// Shorthand for --format json.
    #[arg(long, global = true, conflicts_with = "format")]
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
    let base_root = std::env::current_dir().context("reading current directory")?;
    let report = run_with_base_root(&base_root, &cli)?;
    let format = if cli.json { Format::Json } else { cli.format };
    match format {
        Format::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).context("serializing fetch report")?
            );
        }
        Format::Yml => println!(
            "{}",
            serde_yaml::to_string(&report).context("serializing fetch report to YAML")?
        ),
        Format::Paths => {
            for file in report
                .routes
                .iter()
                .flat_map(|r| {
                    std::iter::once(r.file.as_str())
                        .chain(r.api_calls.iter().map(|f| f.file.as_str()))
                })
                .collect::<BTreeSet<_>>()
            {
                println!("{file}");
            }
        }
        Format::Md | Format::Human => print_markdown_report(&report),
    }
    Ok(ExitCode::SUCCESS)
}
