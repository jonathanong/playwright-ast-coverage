use anyhow::{Context, Result};
use clap::{Args, Subcommand};
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
    /// Output as JSON.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: QueuesCommand,
}

#[derive(Subcommand)]
enum QueuesCommand {
    /// Print queue dependency edges.
    Edges {
        /// Only show edges reachable from these files/nodes.
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
    let report = analyze_project(&root, args.tsconfig.as_deref(), &args.filters)?;
    match &args.command {
        QueuesCommand::Edges { files } => {
            print_edges(&report, files, args.json)?;
            Ok(ExitCode::SUCCESS)
        }
        QueuesCommand::Related { files, direction } => {
            let edges = related(&report, files, (*direction).into());
            print_related(files, &edges, args.json)?;
            Ok(ExitCode::SUCCESS)
        }
        QueuesCommand::Check => {
            print_check(&report.check, args.json)?;
            Ok(if report.check.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
    }
}

fn print_edges(report: &ProjectReport, files: &[String], json: bool) -> Result<()> {
    let edges: Vec<&Edge> = if files.is_empty() {
        report.edges.iter().collect()
    } else {
        report
            .edges
            .iter()
            .filter(|e| files.iter().any(|f| f == &e.from))
            .collect()
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&edges)?);
    } else {
        for edge in &edges {
            println!("{} -> {}", edge.from, edge.to);
        }
    }
    Ok(())
}

fn print_related(roots: &[String], edges: &[Edge], json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(edges)?);
    } else {
        println!("{}", roots.join(", "));
        for edge in edges {
            println!("  {} -> {}", edge.from, edge.to);
        }
    }
    Ok(())
}

fn print_check(findings: &[CheckFinding], json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(findings)?);
    } else {
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
    Ok(())
}
