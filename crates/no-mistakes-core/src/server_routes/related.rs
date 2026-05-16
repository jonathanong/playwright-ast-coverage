use crate::server_routes::graph::RelatedDirection;
use crate::server_routes::model::ProjectReport;
use crate::server_routes::types::Edge;
use std::collections::{HashMap, HashSet, VecDeque};

pub fn related(report: &ProjectReport, roots: &[String], direction: RelatedDirection) -> Vec<Edge> {
    let mut forward: HashMap<String, Vec<Edge>> = HashMap::new();
    let mut reverse: HashMap<String, Vec<Edge>> = HashMap::new();
    for edge in &report.edges {
        forward
            .entry(edge.from.clone())
            .or_default()
            .push(edge.clone());
        reverse.entry(edge.to.clone()).or_default().push(Edge {
            from: edge.to.clone(),
            to: edge.from.clone(),
            kind: edge.kind,
        });
    }
    traverse(roots, direction, &forward, &reverse)
}

fn traverse(
    roots: &[String],
    direction: RelatedDirection,
    forward: &HashMap<String, Vec<Edge>>,
    reverse: &HashMap<String, Vec<Edge>>,
) -> Vec<Edge> {
    let mut seen = HashSet::new();
    let mut queue = VecDeque::new();
    for root in roots {
        seen.insert(root.clone());
        queue.push_back(root.clone());
    }
    let mut out = Vec::new();
    while let Some(node) = queue.pop_front() {
        for edge in neighbors(&node, direction, forward, reverse) {
            if seen.insert(edge.to.clone()) {
                queue.push_back(edge.to.clone());
            }
            out.push(edge);
        }
    }
    out.sort();
    out.dedup();
    out
}

fn neighbors(
    node: &str,
    direction: RelatedDirection,
    forward: &HashMap<String, Vec<Edge>>,
    reverse: &HashMap<String, Vec<Edge>>,
) -> Vec<Edge> {
    let mut edges = Vec::new();
    if matches!(direction, RelatedDirection::Deps | RelatedDirection::Both) {
        edges.extend(forward.get(node).cloned().unwrap_or_default());
    }
    if matches!(
        direction,
        RelatedDirection::Dependents | RelatedDirection::Both
    ) {
        edges.extend(reverse.get(node).cloned().unwrap_or_default());
    }
    edges
}
