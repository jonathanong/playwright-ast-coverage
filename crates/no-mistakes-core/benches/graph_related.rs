use criterion::{criterion_group, criterion_main, Criterion};
use no_mistakes_core::codebase::dependencies::graph::{DepGraph, GraphBuildPlan};
use no_mistakes_core::codebase::dependencies::TsConfig;
use no_mistakes_core::codebase::ts_source::facts::{collect_ts_facts, TsFactPlan};
use no_mistakes_core::queue::related;
use no_mistakes_core::queue::ProjectReport;
use no_mistakes_core::queue::RelatedDirection;
use no_mistakes_core::queue::{Edge, EdgeKind};
use std::hint::black_box;
use std::path::PathBuf;
use tempfile::TempDir;

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

fn create_ts_fixture() -> (TempDir, Vec<PathBuf>, TsConfig) {
    let dir = tempfile::tempdir().expect("benchmark tempdir should be created");
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).expect("benchmark src dir should be created");

    let mut files = Vec::new();
    for i in 0..250 {
        let path = src.join(format!("file_{i}.ts"));
        let next = if i + 1 < 250 {
            format!("import {{ value as next }} from './file_{}';\n", i + 1)
        } else {
            String::new()
        };
        std::fs::write(
            &path,
            format!(
                "{next}export const value = {i};\nexport function f_{i}() {{ return value; }}\n"
            ),
        )
        .expect("benchmark source file should be written");
        files.push(path);
    }

    let tsconfig = TsConfig {
        dir: dir.path().to_path_buf(),
        paths: Vec::new(),
        paths_dir: dir.path().to_path_buf(),
        base_url: None,
    };
    (dir, files, tsconfig)
}

pub fn bench_unified_fact_collection(c: &mut Criterion) {
    let (_dir, files, _tsconfig) = create_ts_fixture();
    c.bench_function("unified_ts_fact_collection", |b| {
        b.iter(|| {
            collect_ts_facts(
                black_box(&files),
                black_box(TsFactPlan::imports_and_symbols()),
            )
        })
    });
}

pub fn bench_dep_graph_build(c: &mut Criterion) {
    let (dir, _files, tsconfig) = create_ts_fixture();
    c.bench_function("dep_graph_full_build", |b| {
        b.iter(|| {
            DepGraph::build_with_plan(
                black_box(dir.path()),
                black_box(&tsconfig),
                black_box(GraphBuildPlan::imports_and_workspace()),
            )
            .expect("benchmark graph should build")
        })
    });
}

criterion_group!(
    benches,
    bench_related,
    bench_unified_fact_collection,
    bench_dep_graph_build
);
criterion_main!(benches);
