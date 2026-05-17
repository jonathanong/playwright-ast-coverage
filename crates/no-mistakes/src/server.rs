use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use no_mistakes_core::server_routes::{
    analyze_project, related, Edge, ProjectReport, RelatedDirection, ServerRoute,
};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Args)]
pub(crate) struct ServerArgs {
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
    command: ServerCommand,
}

#[derive(Subcommand)]
enum ServerCommand {
    /// List extracted server routes.
    Routes {
        /// Only show routes from these files/patterns.
        files: Vec<String>,
    },
    /// Print server route dependency edges.
    Edges {
        /// Only show edges reachable from these files/nodes.
        roots: Vec<String>,
    },
    /// Print files related to the given files via route edges.
    Related {
        #[arg(required = true)]
        roots: Vec<String>,
        #[arg(long, value_enum, default_value = "both")]
        direction: ServerDirection,
    },
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum ServerDirection {
    Deps,
    Dependents,
    Both,
}

impl From<ServerDirection> for RelatedDirection {
    fn from(d: ServerDirection) -> Self {
        match d {
            ServerDirection::Deps => RelatedDirection::Deps,
            ServerDirection::Dependents => RelatedDirection::Dependents,
            ServerDirection::Both => RelatedDirection::Both,
        }
    }
}

pub(crate) fn run(args: ServerArgs) -> Result<ExitCode> {
    let base = std::env::current_dir().context("cwd must be accessible")?;
    let root = if args.root.is_absolute() {
        args.root.clone()
    } else {
        base.join(&args.root)
    };
    let report = analyze_project(&root, args.tsconfig.as_deref(), &args.filters)?;
    match &args.command {
        ServerCommand::Routes { files } => {
            print_routes(&report, files, args.json)?;
        }
        ServerCommand::Edges { roots } => {
            print_edges(&report, roots, args.json)?;
        }
        ServerCommand::Related { roots, direction } => {
            let edges = related(&report, roots, (*direction).into());
            print_related(roots, &edges, args.json)?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn print_routes(report: &ProjectReport, files: &[String], json: bool) -> Result<()> {
    let routes: Vec<&ServerRoute> = if files.is_empty() {
        report.routes.iter().collect()
    } else {
        report
            .routes
            .iter()
            .filter(|r| files.iter().any(|f| f == &r.file || f == &r.route))
            .collect()
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&routes)?);
    } else {
        for route in &routes {
            println!("{} {} -> {}", route.method, route.file, route.route);
        }
    }
    Ok(())
}

fn print_edges(report: &ProjectReport, roots: &[String], json: bool) -> Result<()> {
    let edges: Vec<&Edge> = if roots.is_empty() {
        report.edges.iter().collect()
    } else {
        report
            .edges
            .iter()
            .filter(|e| roots.iter().any(|r| r == &e.from))
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
