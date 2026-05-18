use criterion::{criterion_group, criterion_main, Criterion};
use no_mistakes_core::queue::RelatedDirection;
use no_mistakes_core::queue::ProjectReport;
use no_mistakes_core::queue::{Edge, EdgeKind};
use no_mistakes_core::queue::related;
use std::hint::black_box;

fn create_large_graph() -> ProjectReport {
    let mut edges = Vec::new();
    let num_nodes = 1000;

    for i in 0..num_nodes {
        edges.push(Edge {
            from: format!("node_{}", i),
            to: format!("node_{}", i + 1),
            kind: EdgeKind::QueueEnqueue,
        });

        // Add some branching
        if i % 10 == 0 {
            for j in 1..5 {
                edges.push(Edge {
                    from: format!("node_{}", i),
                    to: format!("node_{}_{}", i, j),
                    kind: EdgeKind::QueueEnqueue,
                });
            }
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

    c.bench_function("related_traversal", |b| {
        b.iter(|| {
            related(
                black_box(&report),
                black_box(&roots),
                black_box(RelatedDirection::Deps),
            )
        })
    });
}

criterion_group!(benches, bench_related);
criterion_main!(benches);
