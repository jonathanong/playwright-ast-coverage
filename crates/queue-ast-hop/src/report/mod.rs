use clap::ValueEnum;
use no_mistakes_core::queue::{Edge, ProjectReport};
use serde::Serialize;

#[derive(Debug, Clone, Copy, Eq, PartialEq, ValueEnum)]
pub(crate) enum Format {
    Json,
    Md,
    Yml,
    Paths,
    Human,
}

pub(crate) fn print_edges(
    report: &ProjectReport,
    roots: &[String],
    _depth: Option<usize>,
    format: Format,
) -> anyhow::Result<()> {
    match format {
        Format::Json => print_json(report),
        Format::Yml => print_yml(report),
        Format::Paths => {
            for edge in &report.edges {
                println!("{}", edge.to);
            }
            Ok(())
        }
        Format::Md => {
            println!("# Queue edges");
            for edge in &report.edges {
                println!("- `{}` -> `{}` ({:?})", edge.from, edge.to, edge.kind);
            }
            Ok(())
        }
        Format::Human => {
            if roots.is_empty() {
                println!("queue edges");
            } else {
                println!("{}", roots.join(", "));
            }
            for edge in &report.edges {
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

fn print_json(report: &ProjectReport) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(report)?);
    Ok(())
}

fn print_yml(report: &ProjectReport) -> anyhow::Result<()> {
    println!("{}", serde_yaml::to_string(report)?);
    Ok(())
}
