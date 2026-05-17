use super::*;

fn queue_fixture_source(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/ast-snippets/ts-queues")
        .join(name);
    std::fs::read_to_string(path).expect("queue fixture source must be readable")
}

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

#[test]
fn detects_create_queue_in_expression_statement_fixture() {
    let source = queue_fixture_source("factory-expression-statement.ts");
    let result = find_create_queue_line(&source, "@factory/pkg", "createQueue");
    assert_eq!(result, Some(3));
}

#[test]
fn detects_create_queue_with_side_effect_import_fixture() {
    let source = queue_fixture_source("factory-side-effect-import.ts");
    let result = find_create_queue_line(&source, "@factory/pkg", "createQueue");
    assert_eq!(result, Some(4));
}

#[test]
fn detects_create_queue_in_variable_declaration_fixture() {
    let source = queue_fixture_source("factory-variable-declaration.ts");
    let result = find_create_queue_line(&source, "@factory/pkg", "createQueue");
    assert_eq!(result, Some(4));
}

#[test]
fn detects_create_queue_in_nested_call_argument_fixture() {
    let source = queue_fixture_source("factory-nested-call.ts");
    let result = find_create_queue_line(&source, "@factory/pkg", "createQueue");
    assert_eq!(result, Some(3));
}

#[test]
fn detects_create_queue_in_casted_fixture() {
    let source = queue_fixture_source("factory-casted-call.ts");
    let result = find_create_queue_line(&source, "@factory/pkg", "createQueue");
    assert_eq!(result, Some(3));
}

#[test]
fn detects_create_queue_in_non_null_fixture() {
    let source = queue_fixture_source("factory-non-null-call.ts");
    let result = find_create_queue_line(&source, "@factory/pkg", "createQueue");
    assert_eq!(result, Some(3));
}

#[test]
fn ignores_non_variable_export_declarations() {
    let source = queue_fixture_source("factory-non-variable-export.ts");
    let result = find_create_queue_line(&source, "@factory/pkg", "createQueue");
    assert_eq!(result, None);
}

#[test]
fn ignores_export_specifier_without_declaration() {
    let source = queue_fixture_source("factory-export-specifier-only.ts");
    let result = find_create_queue_line(&source, "@factory/pkg", "createQueue");
    assert_eq!(result, None);
}

#[test]
fn no_matching_expression_fixture_returns_none() {
    let source = queue_fixture_source("factory-no-matching-expression.ts");
    let result = find_create_queue_line(&source, "@factory/pkg", "createQueue");
    assert_eq!(result, None);
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

#[test]
fn find_queue_name_resolves_expression_statement_fixture() {
    let source = queue_fixture_source("factory-expression-statement.ts");
    let result = find_queue_name(&source, "@factory/pkg", "createQueue");
    assert_eq!(result, Some("expression".to_string()));
}

#[test]
fn find_queue_name_resolves_variable_declaration_fixture() {
    let source = queue_fixture_source("factory-variable-declaration.ts");
    let result = find_queue_name(&source, "@factory/pkg", "createQueue");
    assert_eq!(result, Some("variable".to_string()));
}

#[test]
fn find_queue_name_with_side_effect_import_fixture() {
    let source = queue_fixture_source("factory-side-effect-import.ts");
    let result = find_queue_name(&source, "@factory/pkg", "createQueue");
    assert_eq!(result, Some("side-effect".to_string()));
}

#[test]
fn find_queue_name_resolves_const_binding_fixture() {
    let source = queue_fixture_source("factory-const-bindings.ts");
    let result = find_queue_name(&source, "@factory/pkg", "createQueue");
    assert_eq!(result, Some("constant".to_string()));
}

#[test]
fn find_queue_name_returns_unknown_for_dynamic_argument_fixture() {
    let source = queue_fixture_source("factory-unknown-argument.ts");
    let result = find_queue_name(&source, "@factory/pkg", "createQueue");
    assert_eq!(result, Some("<unknown>".to_string()));
}

#[test]
fn find_queue_name_resolves_casted_fixture() {
    let source = queue_fixture_source("factory-casted-call.ts");
    let result = find_queue_name(&source, "@factory/pkg", "createQueue");
    assert_eq!(result, Some("casted".to_string()));
}

#[test]
fn find_queue_name_resolves_non_null_fixture() {
    let source = queue_fixture_source("factory-non-null-call.ts");
    let result = find_queue_name(&source, "@factory/pkg", "createQueue");
    assert_eq!(result, Some("nonnull".to_string()));
}

#[test]
fn find_queue_name_ignores_export_specifier_without_declaration() {
    let source = queue_fixture_source("factory-export-specifier-only.ts");
    let result = find_queue_name(&source, "@factory/pkg", "createQueue");
    assert_eq!(result, None);
}

#[test]
fn find_queue_name_no_matching_expression_fixture_returns_none() {
    let source = queue_fixture_source("factory-no-matching-expression.ts");
    let result = find_queue_name(&source, "@factory/pkg", "createQueue");
    assert_eq!(result, None);
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

#[test]
fn bfs_reachable_handles_cycles_and_missing_imports() {
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ast-snippets/ts-queues");
    let tsconfig = ts_resolver::TsConfig {
        dir: fixture.clone(),
        paths: vec![],
        paths_dir: fixture.clone(),
        base_url: None,
    };
    let entrypoint = fixture.join("reachable-a.ts");
    let result = bfs_reachable(&entrypoint, &tsconfig);
    assert!(result.iter().any(|path| path.ends_with("reachable-a.ts")));
    assert!(result.iter().any(|path| path.ends_with("reachable-b.ts")));
}

#[test]
fn bfs_reachable_handles_unreadable_start_file() {
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ast-snippets/ts-queues");
    let tsconfig = ts_resolver::TsConfig {
        dir: fixture.clone(),
        paths: vec![],
        paths_dir: fixture.clone(),
        base_url: None,
    };
    let entrypoint = fixture.join("not-created.ts");
    let result = bfs_reachable(&entrypoint, &tsconfig);
    assert_eq!(result.len(), 1);
    assert!(result.iter().any(|path| path.ends_with("not-created.ts")));
}

#[test]
fn factory_helpers_cover_string_imports_missing_inits_and_wrong_callees() {
    let source = r#"
import { "create-queue" as cq, createQueue } from "@factory/pkg";
let uninitialized;
const notString = 1;
const { NAME } = values;
export const alsoMissing;
export const notAQueue = other();
export const exported = cq("string-import");
wrap(member.createQueue("ignored-member"));
wrap(createQueue("nested"));
"#;
    let line = find_create_queue_line(source, "@factory/pkg", "create-queue");
    assert_eq!(line, Some(8));

    let name = find_queue_name(source, "@factory/pkg", "create-queue");
    assert_eq!(name, Some("string-import".to_string()));

    let nested_line = find_create_queue_line(source, "@factory/pkg", "createQueue");
    assert_eq!(nested_line, Some(10));
    let nested_name = find_queue_name(source, "@factory/pkg", "createQueue");
    assert_eq!(nested_name, None);
}

#[test]
fn find_queue_name_covers_exported_const_identifier_and_wrong_factory() {
    let source = r#"
import { createQueue } from "@factory/pkg";
export const QUEUE_NAME = "exported-name";
export const queue = createQueue(QUEUE_NAME);
"#;
    assert_eq!(
        find_queue_name(source, "@factory/pkg", "createQueue"),
        Some("exported-name".to_string())
    );
    assert_eq!(
        find_queue_name(source, "@factory/pkg", "otherFactory"),
        None
    );
}
