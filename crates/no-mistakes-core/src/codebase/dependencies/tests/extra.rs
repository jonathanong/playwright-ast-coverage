use super::super::*;

#[test]
fn http_and_process_relationships_map_to_edge_kinds() {
    let set = relationship_filter(&[RelationshipArg::Http, RelationshipArg::Process]).unwrap();
    assert!(set.contains(&EdgeKind::HttpCall));
    assert!(set.contains(&EdgeKind::ProcessSpawn));
    assert!(relationship_filter(&[RelationshipArg::All]).is_none());
    assert!(relationship_filter(&[]).is_none());
}

#[test]
fn import_only_detection_requires_nonempty_all_import_relationships() {
    assert!(!relationships_are_import_only(&[]));
    assert!(relationships_are_import_only(&[RelationshipArg::Import]));
    assert!(!relationships_are_import_only(&[
        RelationshipArg::Import,
        RelationshipArg::Test,
    ]));
}

#[test]
fn resolve_format_prefers_flags_then_tty_default() {
    assert_eq!(
        resolve_format(true, Some(Format::Human), true),
        Format::Json
    );
    assert_eq!(resolve_format(false, Some(Format::Md), true), Format::Md);
    assert_eq!(resolve_format(false, None, true), Format::Human);
    assert_eq!(resolve_format(false, None, false), Format::Json);
}

#[test]
fn merge_node_entries_keeps_min_depth_and_dedupes_edge_kinds() {
    let node = NodeId::File(PathBuf::from("shared.ts"));
    let mut merged = HashMap::new();
    merge_node_entries(
        &mut merged,
        vec![graph::NodeEntry {
            node: node.clone(),
            depth: 3,
            via: vec![EdgeKind::Import],
        }],
    );
    merge_node_entries(
        &mut merged,
        vec![graph::NodeEntry {
            node: node.clone(),
            depth: 1,
            via: vec![EdgeKind::Import, EdgeKind::TestOf],
        }],
    );

    let entry = merged.get(&node).unwrap();
    assert_eq!(entry.depth, 1);
    assert_eq!(entry.via, vec![EdgeKind::Import, EdgeKind::TestOf]);
}

#[test]
fn deps_direction_rejects_symbol_entrypoints() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis")
        .join("simple");
    let args = TraverseArgs {
        files: vec![PathBuf::from("a.mts#a")],
        root: Some(root),
        tsconfig: None,
        depth: None,
        filters: Vec::new(),
        tests: Vec::new(),
        relationships: Vec::new(),
        format: Some(Format::Json),
        json: false,
        timings: false,
    };

    let err = run(args, Direction::Deps).unwrap_err();

    assert!(err.to_string().contains("#symbol targeting"));
}

fn simple_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis")
        .join("simple")
}

fn symbol_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis")
        .join("symbol-export")
}

fn traverse_args(root: PathBuf, files: Vec<PathBuf>) -> TraverseArgs {
    TraverseArgs {
        files,
        root: Some(root),
        tsconfig: None,
        depth: Some(3),
        filters: Vec::new(),
        tests: Vec::new(),
        relationships: Vec::new(),
        format: Some(Format::Json),
        json: false,
        timings: false,
    }
}

#[test]
fn run_covers_lazy_import_normal_graph_filters_formats_and_timings() {
    let root = simple_root();

    let mut lazy = traverse_args(root.clone(), vec![PathBuf::from("a.mts")]);
    lazy.relationships = vec![RelationshipArg::Import];
    lazy.format = Some(Format::Md);
    lazy.timings = true;
    run(lazy, Direction::Deps).unwrap();

    let mut normal = traverse_args(root.clone(), vec![PathBuf::from("a.mts")]);
    normal.relationships = vec![RelationshipArg::All];
    normal.filters = vec!["*.mts".to_string()];
    normal.tests = vec!["vitest".to_string()];
    normal.format = Some(Format::Yml);
    run(normal, Direction::Deps).unwrap();

    let mut paths = traverse_args(root, vec![PathBuf::from("a.mts")]);
    paths.format = Some(Format::Paths);
    run(paths, Direction::Deps).unwrap();
}

#[test]
fn run_dependents_covers_mixed_symbol_and_plain_entrypoints() {
    let root = symbol_root();
    let mut args = traverse_args(
        root,
        vec![
            PathBuf::from("source.mts#alpha"),
            PathBuf::from("uses-alpha.mts"),
        ],
    );
    args.relationships = vec![RelationshipArg::Import];
    args.format = Some(Format::Human);

    run(args, Direction::Dependents).unwrap();
}
