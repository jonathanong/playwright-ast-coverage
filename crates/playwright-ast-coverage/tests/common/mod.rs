#![allow(dead_code)]

use serde_json::Value;
use std::path::PathBuf;

pub fn fixture(category: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures")
        .join(category)
        .join(name)
}

pub fn has_route_edge(edges: &[Value], route: &str, url: &str) -> bool {
    edges
        .iter()
        .any(|edge| edge["kind"] == "route" && edge["route"] == route && edge["url"] == url)
}
