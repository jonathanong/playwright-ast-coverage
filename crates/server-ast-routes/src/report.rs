use clap::ValueEnum;
use no_mistakes_core::server_routes::{Edge, ProjectReport, ServerRoute};
use serde::Serialize;

#[derive(Debug, Clone, Copy, Eq, PartialEq, ValueEnum)]
pub(crate) enum Format {
    Json,
    Md,
    Yml,
    Paths,
    Human,
}

pub(crate) fn print_routes(
    report: &ProjectReport,
    files: &[String],
    format: Format,
) -> anyhow::Result<()> {
    let routes = route_view(report, files);
    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&routes)?),
        Format::Yml => println!("{}", serde_yaml::to_string(&routes)?),
        Format::Paths => {
            for route in &routes {
                println!("{}", route.route);
            }
        }
        Format::Md => {
            println!("# Server routes");
            for route in &routes {
                println!("- `{}` {} `{}`", route.file, route.method, route.route);
            }
        }
        Format::Human => {
            println!("server routes");
            for route in &routes {
                println!("  {} {} -> {}", route.method, route.file, route.route);
            }
        }
    }
    Ok(())
}

pub(crate) fn print_edges(
    report: &ProjectReport,
    roots: &[String],
    depth: Option<usize>,
    format: Format,
) -> anyhow::Result<()> {
    let edges = edge_view(report, roots, depth);
    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(report)?),
        Format::Yml => println!("{}", serde_yaml::to_string(report)?),
        Format::Paths => {
            for edge in &edges {
                println!("{}", edge.to);
            }
        }
        Format::Md => {
            println!("# Server route edges");
            for edge in &edges {
                println!("- `{}` -> `{}` ({:?})", edge.from, edge.to, edge.kind);
            }
        }
        Format::Human => {
            println!("server route edges");
            for edge in &edges {
                println!("  {} -> {}", edge.from, edge.to);
            }
        }
    }
    Ok(())
}

pub(crate) fn print_related(
    roots: &[String],
    edges: &[Edge],
    format: Format,
) -> anyhow::Result<()> {
    #[derive(Serialize)]
    struct Related<'a> {
        roots: &'a [String],
        edges: &'a [Edge],
    }
    match format {
        Format::Json => println!(
            "{}",
            serde_json::to_string_pretty(&Related { roots, edges })?
        ),
        Format::Yml => println!("{}", serde_yaml::to_string(&Related { roots, edges })?),
        Format::Paths => {
            for edge in edges {
                println!("{}", edge.to);
            }
        }
        Format::Md => {
            println!("# Related server routes");
            for edge in edges {
                println!("- `{}` -> `{}`", edge.from, edge.to);
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

fn route_view(report: &ProjectReport, files: &[String]) -> Vec<ServerRoute> {
    if files.is_empty() {
        return report.routes.clone();
    }
    report
        .routes
        .iter()
        .filter(|route| {
            files
                .iter()
                .any(|file| file == &route.file || file == &route.route)
        })
        .cloned()
        .collect()
}

fn edge_view(report: &ProjectReport, roots: &[String], depth: Option<usize>) -> Vec<Edge> {
    if roots.is_empty() {
        return report.edges.clone();
    }
    let max_depth = depth.unwrap_or(usize::MAX);
    let mut edges = Vec::new();
    let mut frontier = roots
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let mut seen_nodes = frontier.clone();
    let mut seen_edges = std::collections::BTreeSet::new();
    for _ in 0..max_depth {
        let mut next = std::collections::BTreeSet::new();
        for edge in &report.edges {
            if !frontier.contains(&edge.from) {
                continue;
            }
            if seen_edges.insert((edge.from.clone(), edge.to.clone(), edge.kind)) {
                edges.push(edge.clone());
            }
            if seen_nodes.insert(edge.to.clone()) {
                next.insert(edge.to.clone());
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    edges
}
