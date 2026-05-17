pub(crate) use no_mistakes_core::cli::Format;
use no_mistakes_core::cli::{edge_view, root_scoped_edge_depth};
use no_mistakes_core::queue::{Edge, ProjectReport};
use serde::Serialize;

pub(crate) fn print_edges(
    report: &ProjectReport,
    roots: &[String],
    depth: Option<usize>,
    format: Format,
) -> anyhow::Result<()> {
    let depth = root_scoped_edge_depth(roots, depth);
    let edges = edge_view(&report.edges, roots, depth);
    match format {
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(&edges)?);
            Ok(())
        }
        Format::Yml => {
            println!("{}", serde_yaml::to_string(&edges)?);
            Ok(())
        }
        Format::Paths => {
            for edge in &edges {
                println!("{}", edge.to);
            }
            Ok(())
        }
        Format::Md => {
            println!("# Queue edges");
            for edge in &edges {
                println!("- `{}` -> `{}` ({})", edge.from, edge.to, edge.kind);
            }
            Ok(())
        }
        Format::Human => {
            if roots.is_empty() {
                println!("queue edges");
            } else {
                println!("{}", roots.join(", "));
            }
            for edge in &edges {
                println!("  {} -> {}", edge.from, edge.to);
            }
            Ok(())
        }
    }
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
        Format::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&Related { roots, edges })?
            );
        }
        Format::Yml => {
            println!("{}", serde_yaml::to_string(&Related { roots, edges })?);
        }
        Format::Paths => {
            for edge in edges {
                println!("{}", edge.to);
            }
        }
        Format::Md => {
            println!("# Related queue files");
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

pub(crate) fn print_check(report: &ProjectReport, format: Format) -> anyhow::Result<()> {
    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&report.check)?),
        Format::Yml => println!("{}", serde_yaml::to_string(&report.check)?),
        Format::Paths => {
            for finding in &report.check {
                println!("{}", finding.file);
            }
        }
        Format::Md => {
            println!("# Queue check");
            for finding in &report.check {
                println!("- `{}`:{} {}", finding.file, finding.line, finding.message);
            }
        }
        Format::Human => {
            for finding in &report.check {
                println!(
                    "{}[{}] {}:{} {}",
                    finding.kind,
                    finding.job.as_deref().unwrap_or("*"),
                    finding.file,
                    finding.line,
                    finding.message
                );
            }
        }
    }
    Ok(())
}
