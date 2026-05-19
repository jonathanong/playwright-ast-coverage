use crate::analysis::types::{Edge, FetchIndex, TestRef};
use no_mistakes_core::fetch::cache::Cache;
use no_mistakes_core::fetch::resolve::relative_string;
use no_mistakes_core::fetch::route_analysis::collect_route_fetches;
use no_mistakes_core::fetch::types::{FetchOccurrence, FetchSide};
use no_mistakes_core::routes::Route;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

pub(crate) type FetchKey = (String, String);
pub(crate) type FetchCoverageEntry = BTreeMap<
    FetchKey,
    (
        BTreeSet<Arc<String>>,
        BTreeSet<TestRef>,
        BTreeSet<Arc<String>>,
    ),
>;

pub(crate) fn seed_fetch_coverage(fetch_index: &FetchIndex) -> FetchCoverageEntry {
    let mut by_fetch: FetchCoverageEntry = BTreeMap::new();
    for (route_file, fetches) in fetch_index {
        for fetch_occ in fetches {
            if fetch_occ.dynamic || fetch_occ.unsupported {
                continue;
            }
            let key = (fetch_occ.method.clone(), fetch_occ.path.clone());
            by_fetch
                .entry(key)
                .or_insert((Default::default(), Default::default(), Default::default()))
                .2
                .insert(std::sync::Arc::new(route_file.clone()));
        }
    }
    by_fetch
}

pub(crate) fn collect_fetches_for_routes(
    routes: &[Route],
    frontend_root: &Path,
    root: &Path,
) -> anyhow::Result<FetchIndex> {
    let mut cache = Cache {
        files: std::collections::HashMap::new(),
        imports: std::collections::HashMap::new(),
    };
    let mut index = FetchIndex::new();
    for route in routes {
        let fetches = collect_route_fetches(route, frontend_root, root, &mut cache)?;
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
        let Some(fetches) = fetch_index.get(route_file.as_ref()) else {
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
    test_file: &Arc<String>,
    test_name: &Option<Arc<String>>,
    describe_path: &Arc<Vec<String>>,
    route_file: &Arc<String>,
    route: &Arc<String>,
    occ: &FetchOccurrence,
) -> Edge {
    let side = match &occ.side {
        FetchSide::Client => "client",
        FetchSide::Server => "server",
    };
    Edge::Fetch {
        test_file: test_file.clone(),
        test_name: test_name.clone(),
        describe_path: describe_path.clone(),
        route_file: route_file.clone(),
        route: route.clone(),
        method: occ.method.clone(),
        path: occ.path.clone(),
        side: side.to_string(),
        cached: occ.cached,
    }
}
