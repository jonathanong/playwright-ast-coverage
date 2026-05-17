use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use no_mistakes_core::cli::{edge_view, resolve_root, root_scoped_edge_depth, Format};
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
    /// Path to tsconfig.json for path alias resolution.
    #[arg(long, global = true)]
    tsconfig: Option<PathBuf>,
    /// Filter to files matching this glob. Can be repeated.
    #[arg(long = "filter", global = true)]
    filters: Vec<String>,
    /// Maximum edge traversal depth for the edges command when roots are provided.
    /// Defaults to 1 when roots are provided, and unlimited otherwise.
    #[arg(long, alias = "max-depth", global = true)]
    depth: Option<usize>,
    /// Output format: json, yml, md, paths, human.
    #[arg(
        long,
        value_enum,
        default_value = "human",
        global = true,
        conflicts_with = "json"
    )]
    format: Format,
    /// Shorthand for --format json.
    #[arg(long, global = true, conflicts_with = "format")]
    json: bool,
    /// Emit phase timings to stderr.
    #[arg(long, global = true)]
    timings: bool,
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
    let root = resolve_root(&args.root, &base);
    let started = std::time::Instant::now();
    let format = if args.json { Format::Json } else { args.format };
    let report = analyze_project(&root, args.tsconfig.as_deref(), &args.filters)?;
    if args.timings {
        eprintln!(
            "analysis: {:.3}ms",
            started.elapsed().as_secs_f64() * 1000.0
        );
    }
    match &args.command {
        ServerCommand::Routes { files } => {
            print_routes(&report, files, format)?;
        }
        ServerCommand::Edges { roots } => {
            print_edges(&report, roots, args.depth, format)?;
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
        Format::Json => println!("{}", serde_json::to_string_pretty(&routes)?),
        Format::Yml => println!("{}", serde_yaml::to_string(&routes)?),
        Format::Md => {
            println!("# Server routes");
            for route in &routes {
                println!("- `{}` {} `{}`", route.file, route.method, route.route);
            }
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

fn print_edges(
    report: &ProjectReport,
    roots: &[String],
    depth: Option<usize>,
    format: Format,
) -> Result<()> {
    let depth = root_scoped_edge_depth(roots, depth);
    let edges = edge_view(&report.edges, roots, depth);
    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&edges)?),
        Format::Yml => println!("{}", serde_yaml::to_string(&edges)?),
        Format::Md => {
            println!("# Server route edges");
            for edge in &edges {
                println!("- `{}` -> `{}` ({})", edge.from, edge.to, edge.kind);
            }
        }
        Format::Paths => print_edge_paths(&edges),
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
        Format::Json => println!("{}", serde_json::to_string_pretty(edges)?),
        Format::Yml => println!("{}", serde_yaml::to_string(edges)?),
        Format::Md => {
            println!("# Related server routes");
            for edge in edges {
                println!("- `{}` -> `{}`", edge.from, edge.to);
            }
        }
        Format::Paths => print_edge_paths(edges),
        Format::Human => {
            println!("{}", roots.join(", "));
            for edge in edges {
                println!("  {} -> {}", edge.from, edge.to);
            }
        }
    }
    Ok(())
}

fn print_edge_paths(edges: &[Edge]) {
    let paths: BTreeSet<&str> = edges
        .iter()
        .flat_map(|e| [e.from.as_str(), e.to.as_str()])
        .collect();
    for p in paths {
        println!("{p}");
    }
}
