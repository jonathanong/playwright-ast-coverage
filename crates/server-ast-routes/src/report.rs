pub(crate) use no_mistakes_core::cli::Format;
use no_mistakes_core::cli::{edge_view, root_scoped_edge_depth};
use no_mistakes_core::server_routes::{Edge, ProjectReport, ServerRoute};
use serde::Serialize;

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
    #[derive(Serialize)]
    struct Edges<'a> {
        roots: &'a [String],
        depth: Option<usize>,
        edges: &'a [Edge],
    }
    let depth = root_scoped_edge_depth(roots, depth);
    let edges = edge_view(&report.edges, roots, depth);
    match format {
        Format::Json => println!(
            "{}",
            serde_json::to_string_pretty(&Edges {
                roots,
                depth,
                edges: &edges,
            })?
        ),
        Format::Yml => println!(
            "{}",
            serde_yaml::to_string(&Edges {
                roots,
                depth,
                edges: &edges,
            })?
        ),
        Format::Paths => {
            for edge in &edges {
                println!("{}", edge.to);
            }
        }
        Format::Md => {
            println!("# Server route edges");
            for edge in &edges {
                println!("- `{}` -> `{}` ({})", edge.from, edge.to, edge.kind);
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
