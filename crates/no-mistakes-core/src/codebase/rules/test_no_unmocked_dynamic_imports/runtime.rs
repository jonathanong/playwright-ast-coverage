use crate::codebase::dependencies::graph::{DepGraph, EdgeKind, NodeId};
use std::path::{Path, PathBuf};

pub(super) fn runtime_deps(graph: &DepGraph, target: PathBuf) -> Vec<PathBuf> {
    let allowed = [
        EdgeKind::Import,
        EdgeKind::DynamicImport,
        EdgeKind::Require,
        EdgeKind::WorkspaceImport,
    ]
    .into();
    graph
        .deps_of(&[NodeId::File(target)], None, Some(&allowed))
        .into_iter()
        .filter_map(|entry| entry.node.as_file().map(Path::to_path_buf))
        .collect()
}
