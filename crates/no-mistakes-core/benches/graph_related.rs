use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use no_mistakes_core::queue::ProjectReport;
use no_mistakes_core::queue::RelatedDirection;
use no_mistakes_core::queue::{build_graph, related_from_graph, Edge, EdgeKind};

fn create_large_graph() -> ProjectReport {
    let num_nodes = 1000;
    let mut edges = Vec::with_capacity(1400);
    let hub = "node_hotspot".to_string();

    for i in 0..num_nodes {
        let from = format!("node_{}", i);

        edges.push(Edge {
            from: from.clone(),
            to: format!("node_{}", i + 1),
            kind: EdgeKind::QueueEnqueue,
        });

        // Add some branching
        if i % 10 == 0 {
            for j in 1..5 {
                edges.push(Edge {
                    from: from.clone(),
                    to: format!("node_{}_{}", i, j),
                    kind: EdgeKind::QueueEnqueue,
                });
            }
        }

        if i % 10 == 0 {
            edges.push(Edge {
                from,
                to: hub.clone(),
                kind: EdgeKind::QueueEnqueue,
            });
        }

        if i % 25 == 0 {
            edges.push(Edge {
                from: format!("node_{}", i),
                to: hub.clone(),
                kind: EdgeKind::QueueEnqueue,
            });
        }
    }

    ProjectReport {
        producers: Vec::new(),
        workers: Vec::new(),
        jobs: Vec::new(),
        check: Vec::new(),
        edges,
        diagnostics: Vec::new(),
    }
}

pub fn bench_related(c: &mut Criterion) {
    let report = create_large_graph();
    let roots = vec!["node_0".to_string(), "node_500".to_string()];
    let (forward, reverse) = build_graph(&report);

    c.bench_function("related_traversal", |b| {
        b.iter_batched(
            || {},
            |_| {
                black_box(related_from_graph(
                    black_box(&roots),
                    black_box(RelatedDirection::Deps),
                    black_box(&forward),
                    black_box(&reverse),
                ))
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_related);
criterion_main!(benches);
