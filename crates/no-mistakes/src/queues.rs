use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use no_mistakes_core::cli::Format;
use no_mistakes_core::queue::{
    analyze_project, related, CheckFinding, Edge, ProjectReport, RelatedDirection,
};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Args)]
pub(crate) struct QueuesArgs {
    /// Project root directory.
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,
    /// Path to tsconfig.json for path alias resolution.
    #[arg(long, global = true)]
    tsconfig: Option<PathBuf>,
    /// Filter to files matching this glob. Can be repeated.
    #[arg(long = "filter", global = true)]
    filters: Vec<String>,
    /// Output format: json, paths, human.
    #[arg(
        long,
        value_enum,
        default_value = "human",
        global = true,
        conflicts_with = "json"
    )]
    format: Format,
    /// Shorthand for --format json (deprecated, use --format json).
    #[arg(long, global = true, hide = true, conflicts_with = "format")]
    json: bool,
    #[command(subcommand)]
    command: QueuesCommand,
}

#[derive(Subcommand)]
enum QueuesCommand {
    /// Print queue dependency edges.
    Edges {
        /// Only show edges whose source exactly matches these files/nodes.
        files: Vec<String>,
    },
    /// Print files/nodes related to the given files/nodes.
    Related {
        #[arg(required = true)]
        files: Vec<String>,
        #[arg(long, value_enum, default_value = "both")]
        direction: QueueDirection,
    },
    /// Check for unmatched producers and workers.
    Check,
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum QueueDirection {
    Deps,
    Dependents,
    Both,
}

impl From<QueueDirection> for RelatedDirection {
    fn from(d: QueueDirection) -> Self {
        match d {
            QueueDirection::Deps => RelatedDirection::Deps,
            QueueDirection::Dependents => RelatedDirection::Dependents,
            QueueDirection::Both => RelatedDirection::Both,
        }
    }
}

pub(crate) fn run(args: QueuesArgs) -> Result<ExitCode> {
    let base = std::env::current_dir().context("cwd must be accessible")?;
    let root = if args.root.is_absolute() {
        args.root.clone()
    } else {
        base.join(&args.root)
    };
    let format = if args.json { Format::Json } else { args.format };
    let report = analyze_project(&root, args.tsconfig.as_deref(), &args.filters)?;
    match &args.command {
        QueuesCommand::Edges { files } => {
            print_edges(&report, files, format)?;
            Ok(ExitCode::SUCCESS)
        }
        QueuesCommand::Related { files, direction } => {
            let edges = related(&report, files, (*direction).into());
            print_related(files, &edges, format)?;
            Ok(ExitCode::SUCCESS)
        }
        QueuesCommand::Check => {
            print_check(&report.check, format)?;
            Ok(if report.check.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
    }
}

fn print_edges(report: &ProjectReport, files: &[String], format: Format) -> Result<()> {
    let edges: Vec<&Edge> = if files.is_empty() {
        report.edges.iter().collect()
    } else {
        report
            .edges
            .iter()
            .filter(|e| files.iter().any(|f| f == &e.from))
            .collect()
    };
    match format {
        Format::Json | Format::Md | Format::Yml => {
            println!("{}", serde_json::to_string_pretty(&edges)?);
        }
        Format::Paths => {
            for edge in &edges {
                println!("{}", edge.from);
                println!("{}", edge.to);
            }
        }
        Format::Human => {
            for edge in &edges {
                println!("{} -> {}", edge.from, edge.to);
            }
        }
    }
    Ok(())
}

fn print_related(roots: &[String], edges: &[Edge], format: Format) -> Result<()> {
    match format {
        Format::Json | Format::Md | Format::Yml => {
            println!("{}", serde_json::to_string_pretty(edges)?);
        }
        Format::Paths => {
            for edge in edges {
                println!("{}", edge.from);
                println!("{}", edge.to);
            }
        }
        Format::Human => {
            println!("{}", roots.join(", "));
            for edge in edges {
                println!("  {} -> {}", edge.from, edge.to);
            }
        }
    }
    Ok(())
}

fn print_check(findings: &[CheckFinding], format: Format) -> Result<()> {
    match format {
        Format::Json | Format::Md | Format::Yml => {
            println!("{}", serde_json::to_string_pretty(findings)?);
        }
        Format::Paths => {
            for f in findings {
                println!("{}:{}", f.file, f.line);
            }
        }
        Format::Human => {
            for f in findings {
                println!(
                    "{}[{}] {}:{} {}",
                    f.kind,
                    f.job.as_deref().unwrap_or("*"),
                    f.file,
                    f.line,
                    f.message
                );
            }
        }
    }
    Ok(())
}
