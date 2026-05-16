mod extract;
mod extract_helpers;
mod extract_model;
mod extract_record;
mod graph;
mod graph_build;
mod graph_model;
mod graph_related;
mod resolver;
mod source;
mod types;

pub use graph::{analyze_project, RelatedDirection};
pub use graph_model::{CheckFinding, ProjectReport};
pub use graph_related::related;
pub use source::{discover_source_files, relative_string};
pub use types::{Diagnostic, Edge, EdgeKind, QueueJobNode, QueueProducer, QueueWorker, Severity};

#[cfg(test)]
mod tests;
