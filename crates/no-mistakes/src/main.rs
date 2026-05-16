use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use no_mistakes_core::codebase::dependencies::{self, Direction, TraverseArgs};
use no_mistakes_core::codebase::symbols::{self, SymbolsArgs};
use rayon::ThreadPoolBuilder;
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
    /// Dump named exports and imports of TS/JS files.
    Symbols(SymbolsArgs),
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
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    init_threads(cli.jobs);
    match cli.command {
        Command::Dependencies(args) => dependencies::run(args, Direction::Deps),
        Command::Dependents(args) => dependencies::run(args, Direction::Dependents),
        Command::Symbols(args) => symbols::run(args),
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
