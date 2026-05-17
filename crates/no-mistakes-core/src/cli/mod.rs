pub mod output_format;

pub use output_format::Format;

use std::path::{Path, PathBuf};

pub trait TraversableEdge: Clone {
    type Kind: Ord + Clone;

    fn source(&self) -> &str;
    fn target(&self) -> &str;
    fn kind(&self) -> Self::Kind;

    fn identity(&self) -> (String, String, Self::Kind) {
        (
            self.source().to_string(),
            self.target().to_string(),
            self.kind(),
        )
    }
}

impl TraversableEdge for crate::queue::Edge {
    type Kind = crate::queue::EdgeKind;

    fn source(&self) -> &str {
        &self.from
    }

    fn target(&self) -> &str {
        &self.to
    }

    fn kind(&self) -> Self::Kind {
        self.kind
    }
}

impl TraversableEdge for crate::server_routes::Edge {
    type Kind = crate::server_routes::EdgeKind;

    fn source(&self) -> &str {
        &self.from
    }

    fn target(&self) -> &str {
        &self.to
    }

    fn kind(&self) -> Self::Kind {
        self.kind
    }
}

pub fn edge_view<E: TraversableEdge>(
    all_edges: &[E],
    roots: &[String],
    depth: Option<usize>,
) -> Vec<E> {
    if roots.is_empty() {
        return all_edges.to_vec();
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
        for edge in all_edges {
            if !frontier.contains(edge.source()) {
                continue;
            }
            if seen_edges.insert(edge.identity()) {
                edges.push(edge.clone());
            }
            if seen_nodes.insert(edge.target().to_string()) {
                next.insert(edge.target().to_string());
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    edges
}

/// Resolve edge traversal depth for commands that optionally start from roots.
///
/// With no roots, `None` means the full edge list. With roots and no explicit
/// depth, the default is direct edges only (`Some(1)`).
pub fn root_scoped_edge_depth<T>(roots: &[T], depth: Option<usize>) -> Option<usize> {
    if roots.is_empty() {
        depth
    } else {
        Some(depth.unwrap_or(1))
    }
}

#[derive(clap::Args, Debug, Clone, Copy, Default)]
pub struct JobsArg {
    #[arg(
        short = 'j',
        long = "jobs",
        value_name = "N",
        default_value_t = 0,
        global = true
    )]
    pub jobs: usize,
}

pub fn init_rayon_threads(args: JobsArg) {
    let raw_threads = std::env::var("RAYON_NUM_THREADS").ok();
    let threads = rayon_thread_count(args, raw_threads.as_deref());
    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global();
}

fn rayon_thread_count(args: JobsArg, raw_threads: Option<&str>) -> usize {
    if args.jobs > 0 {
        args.jobs
    } else if let Some(raw) = raw_threads {
        raw.parse().unwrap_or_else(|_| num_cpus::get())
    } else {
        num_cpus::get()
    }
}

pub fn resolve_root(root: &Path, cwd: &Path) -> PathBuf {
    if root.is_absolute() {
        root.to_path_buf()
    } else {
        cwd.join(root)
    }
}

pub fn resolve_optional_root(root: Option<&Path>, cwd: &Path) -> PathBuf {
    root.map(|root| resolve_root(root, cwd))
        .unwrap_or_else(|| cwd.to_path_buf())
}

#[cfg(test)]
mod tests;
