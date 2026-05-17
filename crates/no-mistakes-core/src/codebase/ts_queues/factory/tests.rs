use super::*;

// ── collect_import_specifiers ───────────────────────────────────────────

#[test]
fn collects_import_declaration_specifiers() {
    let source = r#"
import { foo } from 'foo-pkg';
import { bar } from './bar.mts';
"#;
    let specs = collect_import_specifiers(source);
    assert!(specs.contains(&"foo-pkg".to_string()));
    assert!(specs.contains(&"./bar.mts".to_string()));
}

#[test]
fn collects_export_named_with_source() {
    let source = "export { foo } from './utils.mts';";
    let specs = collect_import_specifiers(source);
    assert_eq!(specs, vec!["./utils.mts"]);
}

#[test]
fn collects_export_all_with_source() {
    let source = "export * from '@systems/emails/queues';";
    let specs = collect_import_specifiers(source);
    assert_eq!(specs, vec!["@systems/emails/queues"]);
}

#[test]
fn empty_file_yields_no_specifiers() {
    let specs = collect_import_specifiers("");
    assert!(specs.is_empty());
}

// ── find_create_queue_line ──────────────────────────────────────────────

#[test]
fn detects_renamed_import_binding() {
    let source = r#"import { createQueue as cq } from "@factory/pkg";
export const q = cq('name');
"#;
    let result = find_create_queue_line(source, "@factory/pkg", "createQueue");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), 2);
}

#[test]
fn no_create_queue_call_returns_none() {
    let source = r#"import { something } from "@factory/pkg";
export const x = something('name');
"#;
    let result = find_create_queue_line(source, "@factory/pkg", "createQueue");
    assert!(result.is_none());
}

#[test]
fn detects_create_queue_on_correct_line() {
    let source = r#"import { createQueue } from '@factory/glide-mq';
export const emails = createQueue('emails');
"#;
    let result = find_create_queue_line(source, "@factory/glide-mq", "createQueue");
    assert_eq!(result, Some(2));
}

#[test]
fn wrong_factory_specifier_not_detected() {
    let source = r#"import { createQueue } from '@other/pkg';
export const q = createQueue('name');
"#;
    let result = find_create_queue_line(source, "@factory/pkg", "createQueue");
    assert!(result.is_none());
}

// ── find_queue_name with const identifier ────────────────────────────────

#[test]
fn find_queue_name_resolves_const_identifier() {
    let source = r#"import { createQueue } from '@factory/glide-mq';
const QUEUE_NAME = "notifications";
export const q = createQueue(QUEUE_NAME);
"#;
    let result = find_queue_name(source, "@factory/glide-mq", "createQueue");
    assert_eq!(result, Some("notifications".to_string()));
}

#[test]
fn find_queue_name_returns_unknown_for_unresolvable_identifier() {
    let source = r#"import { createQueue } from '@factory/glide-mq';
import { QUEUE_NAME } from './names.mts';
export const q = createQueue(QUEUE_NAME);
"#;
    let result = find_queue_name(source, "@factory/glide-mq", "createQueue");
    assert_eq!(result, Some("<unknown>".to_string()));
}

// ── queue-usage fixture ─────────────────────────────────────────────────

#[test]
fn fixture_queue_usage_extracts_queue_names() {
    // fixtures/queue-usage/queues.mts imports createQueue from a local file
    // and defines three queues: autotagger, email-notifications, image-processing.
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis/queue-usage/queues.mts");
    let source = std::fs::read_to_string(&fixture).expect("fixture file should exist");

    // The factory specifier is the relative path to glide-mq-factory.mts
    let name = find_queue_name(&source, "./glide-mq-factory.mts", "createQueue");
    // find_queue_name returns the first queue name found
    assert!(
        name.is_some(),
        "expected to find a queue name in fixture, got None"
    );
    let n = name.unwrap();
    assert!(
        n == "autotagger" || n == "email-notifications" || n == "image-processing",
        "unexpected queue name: {n}"
    );
}

// ── bfs_reachable ────────────────────────────────────────────────────────

#[test]
fn bfs_reachable_visits_fixture_queue_files() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis/queue-dashboard/good");
    let tsconfig = ts_resolver::TsConfig {
        dir: fixture.clone(),
        paths: vec![
            ("@systems/*".to_string(), vec!["./systems/*".to_string()]),
            ("@example/api/*".to_string(), vec!["./api/*".to_string()]),
        ],
        paths_dir: fixture.clone(),
        base_url: None,
    };
    let entrypoint = fixture.join("server/glidemq-dashboard.mts");
    let result = bfs_reachable(&entrypoint, &tsconfig);
    let names: Vec<_> = result
        .iter()
        .filter_map(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .collect();
    assert!(names.contains(&"glidemq-dashboard.mts".to_string()));
    assert!(names.contains(&"queues.mts".to_string()));
}
