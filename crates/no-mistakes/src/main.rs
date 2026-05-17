use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use no_mistakes_core::codebase::dependencies::{self, Direction, TraverseArgs};
use no_mistakes_core::codebase::symbols::{self, SymbolsArgs};
use no_mistakes_core::react_traits;
use rayon::ThreadPoolBuilder;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(flatten)]
    jobs: JobsArg,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Find files that the given files depend on.
    Dependencies(TraverseArgs),
    /// Find files that depend on the given files.
    Dependents(TraverseArgs),
    /// Find files that depend on the given files (alias for `dependents`).
    Related(TraverseArgs),
    /// Dump named exports and imports of TS/JS files.
    Symbols(SymbolsArgs),
    /// Analyze React component traits.
    React(ReactArgs),
}

#[derive(Args, Debug)]
struct ReactArgs {
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: ReactCommand,
}

#[derive(Subcommand, Debug)]
enum ReactCommand {
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

#[derive(Args, Debug, Clone, Copy, Default)]
struct JobsArg {
    #[arg(
        short = 'j',
        long = "jobs",
        value_name = "N",
        default_value_t = 0,
        global = true
    )]
    jobs: usize,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    init_threads(cli.jobs);
    match cli.command {
        Command::Dependencies(args) => {
            dependencies::run(args, Direction::Deps)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Dependents(args) | Command::Related(args) => {
            dependencies::run(args, Direction::Dependents)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Symbols(args) => {
            symbols::run(args)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::React(args) => run_react(args),
    }
}

fn run_react(args: ReactArgs) -> Result<ExitCode> {
    let ReactArgs {
        root,
        config,
        json,
        command,
    } = args;
    let cwd = std::env::current_dir()?;
    let root = if root.is_absolute() {
        root
    } else {
        cwd.join(root)
    };
    match &command {
        ReactCommand::Analyze { targets } => {
            let results = react_traits::run_analyze(&root, config.as_deref(), targets, None)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&results)
                        .expect("serialization of Rust structs never fails")
                );
            } else {
                react_traits::report::text::print_results(&results, 0);
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
                Ok(ExitCode::SUCCESS)
            } else {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&violations)
                            .expect("serialization of Rust structs never fails")
                    );
                } else {
                    react_traits::report::text::print_violations(&violations);
                }
                Ok(ExitCode::from(1))
            }
        }
    }
}

fn init_threads(args: JobsArg) {
    let threads = if args.jobs > 0 {
        args.jobs
    } else if let Ok(raw) = std::env::var("RAYON_NUM_THREADS") {
        raw.parse().unwrap_or_else(|_| num_cpus::get())
    } else {
        num_cpus::get()
    };
    let _ = ThreadPoolBuilder::new().num_threads(threads).build_global();
}
