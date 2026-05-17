use super::*;
use clap::Parser;

mod extra;

fn parse(argv: &[&str]) -> TraverseArgs {
    TraverseArgs::parse_from(argv)
}

fn build_graph(root: &Path, tsconfig: &crate::codebase::ts_resolver::TsConfig) -> graph::DepGraph {
    let graph_files = graph::GraphFiles::discover(root);
    graph::DepGraph::build_with_plan_and_files(
        root,
        tsconfig,
        graph::GraphBuildPlan::all(),
        &graph_files,
    )
}

fn fixture_root(name: &str) -> PathBuf {
    crate::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis")
            .join(name),
    )
}

#[test]
fn run_surfaces_tsconfig_errors() {
    let root = fixture_root("symbols-output");
    let args = TraverseArgs {
        files: vec![PathBuf::from("src/utils.mts")],
        root: Some(root.clone()),
        tsconfig: Some(root.join("tsconfig-invalid.json")),
        depth: None,
        filters: Vec::new(),
        tests: Vec::new(),
        format: Some(Format::Json),
        json: false,
        relationships: Vec::new(),
        timings: false,
    };

    let err = run(args, Direction::Deps).unwrap_err();

    assert!(format!("{err:#}").contains("tsconfig-invalid.json"));
}

// ── TraverseArgs parsing ────────────────────────────────────────────────

#[test]
fn files_parsed() {
    let a = parse(&["deps", "src/main.mts"]);
    assert_eq!(a.files, vec![PathBuf::from("src/main.mts")]);
    assert!(a.depth.is_none());
    assert!(a.filters.is_empty());
}

#[test]
fn depth_flag_parsed() {
    let a = parse(&["deps", "a.mts", "--depth", "3"]);
    assert_eq!(a.depth, Some(3));
}

#[test]
fn filter_flag_parsed() {
    let a = parse(&["deps", "a.mts", "--filter", "**/*.test.mts"]);
    assert_eq!(a.filters, vec!["**/*.test.mts"]);
}

#[test]
fn filter_flag_repeatable() {
    let a = parse(&[
        "deps",
        "a.mts",
        "--filter",
        "**/*.test.mts",
        "--filter",
        "**/*.spec.mts",
    ]);
    assert_eq!(a.filters.len(), 2);
}

#[test]
fn root_flag_parsed() {
    let a = parse(&["deps", "a.mts", "--root", "/some/path"]);
    assert_eq!(a.root, Some(PathBuf::from("/some/path")));
}

#[test]
fn multiple_input_files_parsed() {
    let a = parse(&["deps", "a.mts", "b.mts", "c.mts"]);
    assert_eq!(a.files.len(), 3);
}

#[test]
fn format_flag_parsed() {
    let a = parse(&["deps", "a.mts", "--format", "md"]);
    assert_eq!(a.format, Some(Format::Md));
}

#[test]
fn format_json_variant() {
    let a = parse(&["deps", "a.mts", "--format", "json"]);
    assert_eq!(a.format, Some(Format::Json));
}

#[test]
fn format_yml_variant() {
    let a = parse(&["deps", "a.mts", "--format", "yml"]);
    assert_eq!(a.format, Some(Format::Yml));
}

#[test]
fn format_paths_variant() {
    let a = parse(&["deps", "a.mts", "--format", "paths"]);
    assert_eq!(a.format, Some(Format::Paths));
}

#[test]
fn format_human_variant() {
    let a = parse(&["deps", "a.mts", "--format", "human"]);
    assert_eq!(a.format, Some(Format::Human));
}

#[test]
fn json_flag_conflicts_with_format() {
    let result = TraverseArgs::try_parse_from(["deps", "a.mts", "--json", "--format", "human"]);
    assert!(result.is_err());
}

#[test]
fn test_flag_parsed() {
    let a = parse(&["deps", "a.mts", "--test", "vitest"]);
    assert_eq!(a.tests, vec!["vitest"]);
}

#[test]
fn test_flag_repeatable() {
    let a = parse(&["deps", "a.mts", "--test", "vitest", "--test", "playwright"]);
    assert_eq!(a.tests.len(), 2);
}

// ── test_globs expansion ────────────────────────────────────────────────

#[test]
fn vitest_globs_include_test_mts() {
    let globs = test_globs("vitest");
    assert!(globs.iter().any(|g| g == "**/*.test.mts"));
    assert!(globs.iter().any(|g| g == "**/*.spec.ts"));
}

#[test]
fn playwright_globs_include_e2e() {
    let globs = test_globs("playwright");
    assert!(globs.contains(&"**/tests/e2e/**/*.mts".to_string()));
    assert!(globs.contains(&"**/playwright/**/*.spec.mts".to_string()));
    assert!(globs.contains(&"**/playwright/**/*.spec.js".to_string()));
}

#[test]
fn cargo_globs_include_tests_dir() {
    let globs = test_globs("cargo");
    assert!(globs.iter().any(|g| g.contains("tests/**/*.rs")));
}

#[test]
fn unknown_framework_returns_empty() {
    let globs = test_globs("jest");
    assert!(globs.is_empty());
}

// ── --relationship / relationship_filter ─────────────────────────────────

#[test]
fn relationship_flag_parsed() {
    let a = parse(&["deps", "a.mts", "--relationship", "import"]);
    assert_eq!(a.relationships, vec![RelationshipArg::Import]);
}

#[test]
fn relationship_flag_repeatable() {
    let a = parse(&[
        "deps",
        "a.mts",
        "--relationship",
        "import",
        "--relationship",
        "test",
    ]);
    assert_eq!(a.relationships.len(), 2);
}

#[test]
fn empty_relationships_returns_none() {
    assert!(relationship_filter(&[]).is_none());
}

#[test]
fn all_keyword_returns_none() {
    assert!(relationship_filter(&[RelationshipArg::All]).is_none());
}

#[test]
fn import_maps_to_all_import_forms() {
    let set = relationship_filter(&[RelationshipArg::Import]).unwrap();
    assert!(set.contains(&EdgeKind::Import));
    assert!(set.contains(&EdgeKind::TypeImport));
    assert!(set.contains(&EdgeKind::DynamicImport));
    assert!(set.contains(&EdgeKind::Require));
    assert!(!set.contains(&EdgeKind::TestOf));
}

#[test]
fn workspace_maps_to_workspace_import() {
    let set = relationship_filter(&[RelationshipArg::Workspace]).unwrap();
    assert!(set.contains(&EdgeKind::WorkspaceImport));
}

#[test]
fn test_maps_to_test_of_and_route_test() {
    let set = relationship_filter(&[RelationshipArg::Test]).unwrap();
    assert!(set.contains(&EdgeKind::TestOf));
    assert!(set.contains(&EdgeKind::RouteTest));
}

#[test]
fn route_maps_to_route_ref_and_route_test() {
    let set = relationship_filter(&[RelationshipArg::Route]).unwrap();
    assert!(set.contains(&EdgeKind::RouteRef));
    assert!(set.contains(&EdgeKind::RouteTest));
}

#[test]
fn queue_maps_to_queue_enqueue_and_queue_worker() {
    let set = relationship_filter(&[RelationshipArg::Queue]).unwrap();
    assert!(set.contains(&EdgeKind::QueueEnqueue));
    assert!(set.contains(&EdgeKind::QueueWorker));
}

#[test]
fn md_maps_to_markdown_link() {
    let set = relationship_filter(&[RelationshipArg::Md]).unwrap();
    assert!(set.contains(&EdgeKind::MarkdownLink));
}

#[test]
fn ci_maps_to_ci_invocation() {
    let set = relationship_filter(&[RelationshipArg::Ci]).unwrap();
    assert!(set.contains(&EdgeKind::CiInvocation));
}

#[test]
fn multiple_kinds_combined() {
    let set = relationship_filter(&[RelationshipArg::Import, RelationshipArg::Test]).unwrap();
    assert!(set.contains(&EdgeKind::Import));
    assert!(set.contains(&EdgeKind::TestOf));
    assert!(!set.contains(&EdgeKind::QueueEnqueue));
    assert!(!set.contains(&EdgeKind::QueueWorker));
}

// ── parse_entrypoint ────────────────────────────────────────────────────

#[test]
fn parse_plain_path() {
    let ep = parse_entrypoint("src/main.mts");
    assert_eq!(ep.file, PathBuf::from("src/main.mts"));
    assert!(ep.symbol.is_none());
}

#[test]
fn parse_path_with_symbol() {
    let ep = parse_entrypoint("src/queues.mts#enqueueBulkTopicEmbeddings");
    assert_eq!(ep.file, PathBuf::from("src/queues.mts"));
    assert_eq!(ep.symbol.as_deref(), Some("enqueueBulkTopicEmbeddings"));
}

#[test]
fn parse_path_multiple_hashes_splits_on_first() {
    let ep = parse_entrypoint("src/foo.mts#sym#extra");
    assert_eq!(ep.symbol.as_deref(), Some("sym#extra"));
}

// ── Integration: build graph from fixture ──────────────────────────────

#[test]
fn deps_fixture_simple_json_output() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis")
        .join("simple");
    let root = crate::codebase::ts_resolver::normalize_path(&root);

    let tsconfig = crate::codebase::ts_resolver::TsConfig {
        dir: root.clone(),
        paths: vec![],
        paths_dir: root.clone(),
        base_url: None,
    };
    let g = build_graph(&root, &tsconfig);
    let a = root.join("a.mts");
    let entries = g.deps_of(&[NodeId::File(a)], None, None);
    assert!(!entries.is_empty(), "a.mts should have deps");

    let mut buf = Vec::new();
    output::write_json(&["a.mts".to_string()], &entries, &root, &mut buf).unwrap();

    let s = String::from_utf8(buf).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    let files = v["files"].as_array().unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f["path"].as_str().unwrap()).collect();
    assert!(paths.contains(&"b.mts"), "b.mts should appear");
    assert!(paths.contains(&"c.mts"), "c.mts should appear");
}

#[test]
fn deps_fixture_format_output() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis")
        .join("format-output");
    let root = crate::codebase::ts_resolver::normalize_path(&root);
    let tsconfig = crate::codebase::ts_resolver::TsConfig {
        dir: root.clone(),
        paths: vec![],
        paths_dir: root.clone(),
        base_url: None,
    };
    let g = build_graph(&root, &tsconfig);
    let a = root.join("a.mts");
    let entries = g.deps_of(&[NodeId::File(a)], None, None);

    // Verify md output contains backtick-quoted paths.
    let mut buf = Vec::new();
    output::write_md(&["a.mts".to_string()], &entries, &root, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("`b.mts`") || s.contains("`c.mts`"));

    // Verify yml output parses correctly.
    let mut buf2 = Vec::new();
    output::write_yml(&["a.mts".to_string()], &entries, &root, &mut buf2).unwrap();
    let s2 = String::from_utf8(buf2).unwrap();
    let v: serde_yaml::Value = serde_yaml::from_str(&s2).unwrap();
    assert!(v["files"].as_sequence().unwrap().len() >= 2);
}

#[test]
fn deps_test_framework_vitest_filter() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis")
        .join("test-framework");
    let root = crate::codebase::ts_resolver::normalize_path(&root);
    let tsconfig = crate::codebase::ts_resolver::TsConfig {
        dir: root.clone(),
        paths: vec![],
        paths_dir: root.clone(),
        base_url: None,
    };
    let g = build_graph(&root, &tsconfig);
    let idx = root.join("src").join("index.mts");
    let entries = g.dependents_of(&[NodeId::File(idx)], None, None);

    let mut filters = test_globs("vitest");
    filters.extend(test_globs("playwright"));
    let filter_spec = graph::build_filter(&filters).unwrap().unwrap();
    let filtered = graph::apply_filter(entries, Some(&filter_spec), &root);

    let paths: Vec<_> = filtered
        .iter()
        .filter_map(|e| e.node.as_file())
        .map(|p| p.to_str().unwrap())
        .collect();
    assert!(
        paths.iter().any(|p| p.ends_with("index.test.mts")),
        "vitest test should be included"
    );
}

#[test]
fn filter_fixture_excludes_test_files() {
    // fixtures/filter/src/main.mts imports both utils.mts and utils.test.mts.
    // With a glob filter of "**/*.test.mts", only test files should appear.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis")
        .join("filter");
    let root = crate::codebase::ts_resolver::normalize_path(&root);
    let tsconfig = crate::codebase::ts_resolver::TsConfig {
        dir: root.clone(),
        paths: vec![],
        paths_dir: root.clone(),
        base_url: None,
    };
    let g = build_graph(&root, &tsconfig);
    let main = root.join("src").join("main.mts");
    let entries = g.deps_of(&[NodeId::File(main)], None, None);
    assert!(!entries.is_empty(), "main.mts should have deps");

    let filter_spec = graph::build_filter(&["**/*.test.mts".to_string()])
        .unwrap()
        .unwrap();
    let filtered = graph::apply_filter(entries, Some(&filter_spec), &root);

    let paths: Vec<_> = filtered
        .iter()
        .filter_map(|e| e.node.as_file())
        .map(|p| p.file_name().unwrap().to_str().unwrap())
        .collect();
    assert!(
        paths.iter().all(|p| p.ends_with(".test.mts")),
        "filter should only return .test.mts files, got: {:?}",
        paths
    );
    assert!(
        paths.contains(&"utils.test.mts"),
        "expected utils.test.mts in filtered output, got: {:?}",
        paths
    );
}

#[test]
fn symbol_export_fixture_alpha_dependents() {
    // fixtures/symbol-export/source.mts exports alpha, beta, gamma.
    // uses-alpha.mts and reexport.mts both import alpha.
    // uses-beta.mts imports beta. uses-all.mts imports all three.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis")
        .join("symbol-export");
    let root = crate::codebase::ts_resolver::normalize_path(&root);
    let tsconfig = crate::codebase::ts_resolver::TsConfig {
        dir: root.clone(),
        paths: vec![],
        paths_dir: root.clone(),
        base_url: None,
    };
    let g = build_graph(&root, &tsconfig);
    let source = root.join("source.mts");
    let entries = g.dependents_of(&[NodeId::File(source)], None, None);
    assert!(!entries.is_empty(), "source.mts should have dependents");

    let paths: Vec<_> = entries
        .iter()
        .filter_map(|e| e.node.as_file())
        .filter_map(|p| p.file_name())
        .map(|n| n.to_str().unwrap())
        .collect();
    // uses-alpha, uses-beta, uses-all, reexport, consumer all ultimately depend on source.
    assert!(
        paths.contains(&"uses-alpha.mts"),
        "expected uses-alpha.mts in dependents of source.mts, got: {:?}",
        paths
    );
    assert!(
        paths.contains(&"uses-beta.mts"),
        "expected uses-beta.mts in dependents of source.mts, got: {:?}",
        paths
    );
}

#[test]
fn folder_suffix_fixture() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis")
        .join("folder-suffix");
    let root = crate::codebase::ts_resolver::normalize_path(&root);
    let tsconfig = crate::codebase::ts_resolver::TsConfig {
        dir: root.clone(),
        paths: vec![],
        paths_dir: root.clone(),
        base_url: None,
    };
    let g = build_graph(&root, &tsconfig);
    let main = root.join("main.mts");
    let entries = g.deps_of(&[NodeId::File(main)], None, None);

    let spec = graph::build_filter(&["backend/systems/*/".to_string()])
        .unwrap()
        .unwrap();
    let filtered = graph::apply_filter(entries, Some(&spec), &root);

    // Should collapse to 3 folder entries: emails, users, search.
    assert_eq!(
        filtered.len(),
        3,
        "expected 3 folders, got {:?}",
        filtered
            .iter()
            .filter_map(|e| e.node.as_file())
            .map(|p| p.to_str().unwrap())
            .collect::<Vec<_>>()
    );
}
