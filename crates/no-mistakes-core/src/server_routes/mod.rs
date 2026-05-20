mod extract;
mod graph;
mod model;
mod mounts;
mod normalize;
mod related;
mod source;
mod types;

pub(crate) use graph::route_defs_from_files;
pub use graph::{analyze_project, RelatedDirection};
pub use model::ProjectReport;
pub use related::related;
pub use types::{Diagnostic, Edge, EdgeKind, Framework, ServerRoute, Severity, Summary};

#[cfg(test)]
mod tests;
