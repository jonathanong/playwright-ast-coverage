use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use no_mistakes_core::cli::Format;
use no_mistakes_core::react_traits;
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
    /// Output format: json, yml, md, paths, human.
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
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Analyze component traits and print results.
    Analyze {
        #[arg(help = "Glob patterns for component files")]
        targets: Vec<String>,
    },
    /// Check for violations such as components that call fetch.
    Check {
        #[arg(help = "Glob patterns for component files")]
        targets: Vec<String>,
        /// Exit non-zero if any component or rendered subtree calls fetch.
        #[arg(long)]
        assert_no_fetch: bool,
    },
}

#[cfg(test)]
fn parse_cli_args() -> Cli {
    let raw_args = std::env::var("REACT_TRAITS_TEST_ARGS")
        .expect("REACT_TRAITS_TEST_ARGS must be set in tests - use with_run_args_env()");
    Cli::parse_from(raw_args.split('\u{1f}'))
}

#[cfg(not(test))]
fn parse_cli_args() -> Cli {
    Cli::parse()
}

pub fn run_cli() -> Result<ExitCode> {
    let cli = parse_cli_args();
    let base_root = std::env::current_dir().context("reading current directory")?;
    let root = base_root.join(&cli.root);
    let format = if cli.json { Format::Json } else { cli.format };
    match &cli.command {
        Command::Analyze { targets } => {
            let results = react_traits::run_analyze(&root, cli.config.as_deref(), targets, None)?;
            match format {
                Format::Json => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&results)
                            .context("serializing analysis results")?
                    );
                }
                Format::Yml => {
                    println!(
                        "{}",
                        serde_yaml::to_string(&results)
                            .context("serializing analysis results to YAML")?
                    );
                }
                Format::Md => react_traits::print_results_md(&results),
                Format::Paths => {
                    for result in &results {
                        println!("{}", result.file);
                    }
                }
                Format::Human => react_traits::print_results(&results, 0),
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
                match format {
                    Format::Json => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&violations)
                                .context("serializing check violations")?
                        );
                    }
                    Format::Yml => {
                        println!(
                            "{}",
                            serde_yaml::to_string(&violations)
                                .context("serializing check violations to YAML")?
                        );
                    }
                    Format::Md => react_traits::print_violations_md(&violations),
                    Format::Paths => {
                        for violation in &violations {
                            println!("{}", violation.file);
                        }
                    }
                    Format::Human => react_traits::print_violations(&violations),
                }
                Ok(ExitCode::from(1))
            }
        }
    }
}
