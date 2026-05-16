use crate::analysis::types::{Edge, FetchIndex};
use no_mistakes_core::fetch::cache::Cache;
use no_mistakes_core::fetch::resolve::relative_string;
use no_mistakes_core::fetch::route_analysis::collect_route_fetches;
use no_mistakes_core::fetch::types::{FetchOccurrence, FetchSide};
use no_mistakes_core::routes::Route;
use std::path::Path;

pub(crate) fn collect_fetches_for_routes(
    routes: &[Route],
    frontend_root: &Path,
    root: &Path,
    cache: &mut Cache,
) -> anyhow::Result<FetchIndex> {
    let mut index = FetchIndex::new();
    for route in routes {
        let fetches = collect_route_fetches(route, frontend_root, root, cache)?;
        let rel_file = relative_string(root, &route.file);
        index.insert(rel_file, fetches);
    }
    Ok(index)
}

pub(crate) fn expand_fetch_edges(edges: &[Edge], fetch_index: &FetchIndex) -> Vec<Edge> {
    let mut fetch_edges = Vec::new();
    for edge in edges {
        let Edge::Route {
            test_file,
            test_name,
            describe_path,
            route_file,
            route,
            ..
        } = edge
        else {
            continue;
        };
        let Some(fetches) = fetch_index.get(route_file) else {
            continue;
        };
        for fetch_occ in fetches {
            if fetch_occ.dynamic || fetch_occ.unsupported {
                continue;
            }
            fetch_edges.push(fetch_edge(
                test_file,
                test_name,
                describe_path,
                route_file,
                route,
                fetch_occ,
            ));
        }
    }
    fetch_edges
}

fn fetch_edge(
    test_file: &str,
    test_name: &Option<String>,
    describe_path: &[String],
    route_file: &str,
    route: &str,
    occ: &FetchOccurrence,
) -> Edge {
    let side = match &occ.side {
        FetchSide::Client => "client",
        FetchSide::Server => "server",
    };
    Edge::Fetch {
        test_file: test_file.to_string(),
        test_name: test_name.clone(),
        describe_path: describe_path.to_vec(),
        route_file: route_file.to_string(),
        route: route.to_string(),
        method: occ.method.clone(),
        path: occ.path.clone(),
        side: side.to_string(),
        cached: occ.cached,
    }
}
