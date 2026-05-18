use no_mistakes_core::codebase::check_facts::{collect_check_facts, CheckFactPlan};
use no_mistakes_core::codebase::ts_source::discover_files;
use no_mistakes_core::codebase::unique_exports;
use no_mistakes_core::queue;
use std::path::PathBuf;

fn codebase_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis")
        .join(name)
}

fn queue_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/queue-ast-hop")
        .join(name)
}

#[test]
fn unique_exports_public_api_uses_shared_facts() {
    let root = codebase_fixture("unique-exports-basic");

    let findings = unique_exports::analyze_project(&root, None, None).unwrap();

    assert_eq!(findings.len(), 2);
}

#[test]
fn queue_public_api_uses_shared_facts() {
    let root = queue_fixture("basic");
    let facts = collect_check_facts(
        &root,
        discover_files(&root, &[]),
        CheckFactPlan {
            queue: true,
            ..Default::default()
        },
    );

    let report = queue::analyze_project_with_facts(&root, None, &[], &facts).unwrap();

    assert_eq!(report.check, vec![]);
    assert!(report
        .edges
        .iter()
        .any(|edge| edge.from == "enqueue.ts" && edge.to == "queues.ts#sendWelcome"));
}
