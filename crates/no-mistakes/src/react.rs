use anyhow::Result;
use clap::{Args, Subcommand};
use no_mistakes_core::cli::Format;
use no_mistakes_core::react_traits;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Args, Debug)]
pub(crate) struct ReactArgs {
    #[arg(long, default_value = ".", global = true)]
    pub(crate) root: PathBuf,
    #[arg(long, global = true)]
    pub(crate) config: Option<PathBuf>,
    /// Output format: json, paths, human.
    #[arg(
        long,
        value_enum,
        default_value = "human",
        global = true,
        conflicts_with = "json"
    )]
    pub(crate) format: Format,
    /// Shorthand for --format json (deprecated, use --format json).
    #[arg(long, global = true, hide = true, conflicts_with = "format")]
    pub(crate) json: bool,
    #[command(subcommand)]
    pub(crate) command: ReactCommand,
}

#[derive(Subcommand, Debug)]
pub(crate) enum ReactCommand {
    /// Analyze component traits and print results.
    Analyze {
        #[arg(help = "Glob patterns for component files")]
        targets: Vec<String>,
    },
    /// Check for violations (e.g. assert-no-fetch).
    Check {
        #[arg(help = "Glob patterns for component files")]
        targets: Vec<String>,
        #[arg(long)]
        assert_no_fetch: bool,
    },
}

pub(crate) fn run(args: ReactArgs) -> Result<ExitCode> {
    let ReactArgs {
        root,
        config,
        format,
        json,
        command,
    } = args;
    let effective_format = if json { Format::Json } else { format };
    let cwd = std::env::current_dir()?;
    let root = if root.is_absolute() {
        root
    } else {
        cwd.join(root)
    };
    match &command {
        ReactCommand::Analyze { targets } => {
            let results = react_traits::run_analyze(&root, config.as_deref(), targets, None)?;
            if matches!(effective_format, Format::Json | Format::Md | Format::Yml) {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&results)
                        .expect("serialization of Rust structs never fails")
                );
            } else {
                react_traits::print_results(&results, 0);
            }
            Ok(ExitCode::SUCCESS)
        }
        ReactCommand::Check {
            targets,
            assert_no_fetch,
        } => {
            let violations =
                react_traits::run_check(&root, config.as_deref(), targets, *assert_no_fetch)?;
            if violations.is_empty() {
                return Ok(ExitCode::SUCCESS);
            }
            if matches!(effective_format, Format::Json | Format::Md | Format::Yml) {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&violations)
                        .expect("serialization of Rust structs never fails")
                );
            } else {
                react_traits::print_violations(&violations);
            }
            Ok(ExitCode::from(1))
        }
    }
}
