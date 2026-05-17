mod check;
mod queues;
mod react;
mod server;

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
    /// Find files that depend on the given files (alias for `dependents`).
    Related(TraverseArgs),
    /// Dump named exports and imports of TS/JS files.
    Symbols(SymbolsArgs),
    /// Analyze React component traits.
    React(react::ReactArgs),
    /// Analyze queue producer/worker relationships (BullMQ, glide-mq).
    Queues(queues::QueuesArgs),
    /// Analyze server route graphs (Express, Hono, Koa).
    Server(server::ServerArgs),
    /// Run all checks across configured projects (react + queues).
    Check(check::CheckArgs),
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
        Command::React(args) => react::run(args),
        Command::Queues(args) => queues::run(args),
        Command::Server(args) => server::run(args),
        Command::Check(args) => check::run(args),
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
