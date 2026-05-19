use super::*;
use crate::queue::extract::extract_file;
use crate::queue::extract_helpers::quoted_prefix;
use crate::queue::extract_model::FileFacts;
use crate::queue::graph_model::diagnostics;
use crate::queue::resolver::{load_tsconfig, resolve_import};
use crate::queue::source::discover_source_files;
use std::collections::HashMap;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/queue-ast-hop")
        .join(name)
}

#[test]
fn basic_project_reports_queue_edges() {
    let report = analyze_project(&fixture("basic"), None, &[]).unwrap();
    assert_eq!(report.check, vec![]);
    assert!(report
        .edges
        .iter()
        .any(|edge| edge.from == "enqueue.ts" && edge.to == "queues.ts#sendWelcome"));
    assert!(report
        .edges
        .iter()
        .any(|edge| edge.from == "queues.ts#sendWelcome" && edge.to == "worker.ts"));
}

#[test]
fn shared_facts_project_reports_queue_edges() {
    let root = fixture("basic");
    let files = crate::codebase::ts_source::discover_files(&root, &[]);
    let facts = crate::codebase::check_facts::collect_check_facts(
        &root,
        files,
        crate::codebase::check_facts::CheckFactPlan {
            queue: true,
            ..Default::default()
        },
    );

    let report = analyze_project_with_facts(&root, None, &[], &facts).unwrap();

    assert_eq!(report.check, vec![]);
    assert!(report
        .edges
        .iter()
        .any(|edge| edge.from == "enqueue.ts" && edge.to == "queues.ts#sendWelcome"));
    assert!(report
        .edges
        .iter()
        .any(|edge| edge.from == "queues.ts#sendWelcome" && edge.to == "worker.ts"));
}

#[test]
fn missing_project_root_returns_empty_report() {
    let report = analyze_project(&fixture("does-not-exist"), None, &[]).unwrap();
    assert!(report.edges.is_empty());
    assert!(report.producers.is_empty());
    assert!(report.workers.is_empty());
}

#[test]
fn dynamic_producer_is_warning_not_check_failure() {
    let report = analyze_project(&fixture("dynamic"), None, &[]).unwrap();
    assert!(report
        .check
        .iter()
        .any(|finding| finding.kind == "unmatched-worker"));
    assert!(report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("dynamic")));
}

#[test]
fn flow_producer_edges_are_supported() {
    let report = analyze_project(&fixture("flow"), None, &[]).unwrap();
    assert_eq!(report.check, vec![]);
    assert!(report
        .edges
        .iter()
        .any(|edge| edge.from == "flow.ts" && edge.to == "flow.ts#resize"));
    assert!(report
        .producers
        .iter()
        .any(|producer| producer.job.as_deref() == Some("resize")
            && producer.raw_job.as_deref() == Some("JOB")));
}

#[test]
fn tsconfig_paths_resolve_queue_imports() {
    let root = fixture("tsconfig-paths");
    let report = analyze_project(&root, Some(&root.join("tsconfig.json")), &[]).unwrap();
    assert_eq!(report.check, vec![]);
    assert!(report
        .producers
        .iter()
        .any(|producer| producer.queue_name.as_deref() == Some("email-paths")));
}

#[test]
fn related_crosses_virtual_queue_jobs() {
    let report = analyze_project(&fixture("basic"), None, &[]).unwrap();
    let edges = related(&report, &["enqueue.ts".to_string()], RelatedDirection::Both);
    assert!(edges.iter().any(|edge| edge.to == "queues.ts#sendWelcome"));
    assert!(edges.iter().any(|edge| edge.to == "worker.ts"));
}

#[test]
fn add_bulk_and_wildcard_worker_are_supported() {
    let report = analyze_project(&fixture("bulk"), None, &[]).unwrap();
    assert_eq!(report.check, vec![]);
    assert_eq!(report.jobs.len(), 2);
    assert!(report.workers.iter().any(|worker| worker.wildcard));
}

#[test]
fn unmatched_static_producer_and_worker_are_check_findings() {
    let report = analyze_project(&fixture("unmatched"), None, &[]).unwrap();
    assert!(report
        .check
        .iter()
        .any(|finding| finding.kind == "unmatched-producer"));
    assert!(report
        .check
        .iter()
        .any(|finding| finding.kind == "unmatched-worker"));
}

#[test]
fn filters_limit_discovered_sources() {
    let report = analyze_project(&fixture("basic"), None, &["enqueue.ts".to_string()]).unwrap();
    assert!(report.edges.is_empty());
    assert!(report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("producer")));
}

#[test]
fn missing_tsconfig_returns_error() {
    let err = analyze_project(
        &fixture("basic"),
        Some(&fixture("basic").join("missing.json")),
        &[],
    )
    .unwrap_err();
    assert!(err.to_string().contains("No such file") || err.to_string().contains("os error"));
}

#[test]
fn alternate_syntaxes_and_dynamic_sites_are_recorded() {
    let root = fixture("syntax");
    let report = analyze_project(&root, None, &[]).unwrap();
    assert!(report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("producer")));
    assert!(report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("worker")));
    assert!(report
        .workers
        .iter()
        .any(|worker| worker.processor_file.as_deref() == Some("processor.ts")));
    assert!(report
        .producers
        .iter()
        .any(|producer| producer.raw_job.as_deref() == Some("JOB")));
    assert!(report
        .producers
        .iter()
        .any(|producer| producer.raw_job.as_deref() == Some("DYNAMIC_JOB")));
}

#[test]
fn diagnostics_include_parser_warnings_from_facts() {
    let root = fixture("basic");
    let path = root.join("enqueue.ts");
    let mut facts = FileFacts::default();
    facts.diagnostics.push((7, "synthetic warning".to_string()));
    let diagnostics = diagnostics(&root, &HashMap::from([(path, facts)]), &[], &[]);
    assert_eq!(diagnostics[0].line, 7);
    assert_eq!(diagnostics[0].message, "synthetic warning");
}

#[test]
fn extract_file_records_import_forms_without_queue_semantics() {
    let root = fixture("syntax");
    let facts = extract_file(&root.join("imports.ts"), &root).unwrap();
    assert!(facts
        .imports
        .iter()
        .any(|import| import.imported == "default" && import.local == "Bull"));
    assert!(facts
        .imports
        .iter()
        .any(|import| import.imported == "legacyName" && import.local == "legacy"));
}

#[test]
fn resolver_handles_exact_paths_base_url_and_indexes() {
    let root = fixture("resolver");
    let tsconfig = load_tsconfig(&root, Some(&root.join("tsconfig.json"))).unwrap();
    let current = root.join("src/enqueue.ts");
    assert_eq!(
        resolve_import("@queues", &current, &root, &tsconfig),
        Some(root.join("src/queues/index.ts").canonicalize().unwrap())
    );
    assert_eq!(
        resolve_import("src/processors/worker", &current, &root, &tsconfig),
        Some(
            root.join("src/processors/worker.ts")
                .canonicalize()
                .unwrap()
        )
    );
    assert_eq!(
        resolve_import("@queue-dir", &current, &root, &tsconfig),
        Some(root.join("src/queues/index.ts"))
    );
    assert_eq!(
        resolve_import("./direct.ts", &current, &root, &tsconfig),
        Some(root.join("src/direct.ts").canonicalize().unwrap())
    );
}

#[test]
fn resolver_accepts_jsonc_tsconfig() {
    let root = fixture("resolver");
    let tsconfig = load_tsconfig(&root, Some(&root.join("tsconfig-jsonc.json"))).unwrap();
    let current = root.join("src/enqueue.ts");
    assert_eq!(
        resolve_import("@queues", &current, &root, &tsconfig),
        Some(root.join("src/queues/index.ts").canonicalize().unwrap())
    );
}

#[test]
fn quoted_prefix_returns_partial_value_when_closing_quote_is_absent() {
    assert_eq!(quoted_prefix("\"partial"), Some("partial".to_string()));
}

#[test]
fn resolver_handles_relative_tsconfig_and_fallback_targets() {
    let root = fixture("resolver");
    let tsconfig = load_tsconfig(&root, Some(std::path::Path::new("tsconfig.json"))).unwrap();
    let current = root.join("src/enqueue.ts");
    assert_eq!(
        resolve_import("@fallback/worker", &current, &root, &tsconfig),
        Some(
            root.join("src/processors/worker.ts")
                .canonicalize()
                .unwrap()
        )
    );
}

#[test]
fn resolver_defaults_when_no_tsconfig_exists() {
    let config = load_tsconfig(&fixture("basic"), None).unwrap();
    assert!(config.base_url.is_none());
    assert!(config.paths.is_empty());
}

#[test]
fn invalid_tsconfig_returns_parse_error() {
    let root = fixture("invalid-tsconfig");
    let err = load_tsconfig(&root, None).unwrap_err();
    assert!(!err.to_string().is_empty());
}

#[test]
fn discovery_skips_dependency_and_build_directories() {
    let root = fixture("syntax");
    let files = discover_source_files(&root);
    for skipped in ["node_modules", "target", "build"] {
        assert!(
            !files
                .iter()
                .any(|file| file.to_string_lossy().contains(skipped)),
            "discovered file under {skipped}"
        );
    }
}

#[test]
fn shared_facts_filter_excludes_non_matching_files() {
    let root = fixture("basic");
    let files = crate::codebase::ts_source::discover_files(&root, &[]);
    let facts = crate::codebase::check_facts::collect_check_facts(
        &root,
        files,
        crate::codebase::check_facts::CheckFactPlan {
            queue: true,
            ..Default::default()
        },
    );

    // Filter to only worker.ts so that enqueue.ts and queues.ts are excluded.
    // This causes the `continue` branch inside the filter block (graph.rs lines 62-64)
    // to execute for each skipped file.
    let report =
        analyze_project_with_facts(&root, None, &["worker.ts".to_string()], &facts).unwrap();

    // With no queue definitions visible (queues.ts was filtered out) the worker cannot
    // resolve its queue, so job_keys() returns empty and no edges or check findings appear.
    assert!(report.edges.is_empty());
    assert!(report.check.is_empty());
    // worker.ts itself is still present in the workers list.
    assert!(!report.workers.is_empty());
}
