use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use no_mistakes_core::cli::Format;
use no_mistakes_core::server_routes::{
    analyze_project, related, Edge, ProjectReport, RelatedDirection, ServerRoute,
};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Args)]
pub(crate) struct ServerArgs {
    /// Project root directory.
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,
    /// Filter to files matching this glob. Can be repeated.
    #[arg(long = "filter", global = true)]
    filters: Vec<String>,
    /// Output format: json, paths, human (md/yml use JSON serialization).
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
    command: ServerCommand,
}

#[derive(Subcommand)]
enum ServerCommand {
    /// List extracted server routes.
    Routes {
        /// Only show routes whose file or route exactly matches one of these values.
        files: Vec<String>,
    },
    /// Print server route dependency edges.
    Edges {
        /// Only show edges whose source exactly matches these files/nodes.
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
    let format = if args.json { Format::Json } else { args.format };
    let report = analyze_project(&root, None, &args.filters)?;
    match &args.command {
        ServerCommand::Routes { files } => {
            print_routes(&report, files, format)?;
        }
        ServerCommand::Edges { roots } => {
            print_edges(&report, roots, format)?;
        }
        ServerCommand::Related { roots, direction } => {
            let edges = related(&report, roots, (*direction).into());
            print_related(roots, &edges, format)?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn print_routes(report: &ProjectReport, files: &[String], format: Format) -> Result<()> {
    let routes: Vec<&ServerRoute> = if files.is_empty() {
        report.routes.iter().collect()
    } else {
        report
            .routes
            .iter()
            .filter(|r| files.iter().any(|f| f == &r.file || f == &r.route))
            .collect()
    };
    match format {
        Format::Json | Format::Md | Format::Yml => {
            println!("{}", serde_json::to_string_pretty(&routes)?);
        }
        Format::Paths => {
            let files: BTreeSet<&str> = routes.iter().map(|r| r.file.as_str()).collect();
            for f in files {
                println!("{f}");
            }
        }
        Format::Human => {
            for route in &routes {
                println!("{} {} -> {}", route.method, route.file, route.route);
            }
        }
    }
    Ok(())
}

fn print_edges(report: &ProjectReport, roots: &[String], format: Format) -> Result<()> {
    let edges: Vec<&Edge> = if roots.is_empty() {
        report.edges.iter().collect()
    } else {
        report
            .edges
            .iter()
            .filter(|e| roots.iter().any(|r| r == &e.from))
            .collect()
    };
    match format {
        Format::Json | Format::Md | Format::Yml => {
            println!("{}", serde_json::to_string_pretty(&edges)?);
        }
        Format::Paths => {
            let paths: BTreeSet<&str> = edges
                .iter()
                .flat_map(|e| [e.from.as_str(), e.to.as_str()])
                .collect();
            for p in paths {
                println!("{p}");
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
            let paths: BTreeSet<&str> = edges
                .iter()
                .flat_map(|e| [e.from.as_str(), e.to.as_str()])
                .collect();
            for p in paths {
                println!("{p}");
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
