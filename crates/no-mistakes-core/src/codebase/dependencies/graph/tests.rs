use super::*;
use std::process::Command;
use tempfile::TempDir;

fn p(s: &str) -> PathBuf {
    PathBuf::from(s)
}

fn n(s: &str) -> NodeId {
    NodeId::File(p(s))
}

fn raw_fwd(pairs: &[(&str, &[&str])]) -> HashMap<PathBuf, Vec<PathBuf>> {
    pairs
        .iter()
        .map(|(k, vs)| (p(k), vs.iter().map(|v| p(v)).collect()))
        .collect()
}

fn raw_rev(pairs: &[(&str, &[&str])]) -> HashMap<PathBuf, Vec<PathBuf>> {
    raw_fwd(pairs)
}

fn mk_entry(path: &str, depth: usize) -> NodeEntry {
    NodeEntry {
        node: NodeId::File(p(path)),
        depth,
        via: vec![],
    }
}

fn git_init(dir: &Path) {
    let output = Command::new("git")
        .args(["init", "-q", "--initial-branch=main"])
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_add_all(dir: &Path) {
    let output = Command::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git add failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write(dir: &Path, rel: &str, content: &str) -> PathBuf {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, content).unwrap();
    path
}

// ── bfs ─────────────────────────────────────────────────────────────────

#[test]
fn bfs_linear_chain() {
    let mut fwd: EdgeMap = HashMap::new();
    fwd.insert(n("/a"), vec![(n("/b"), EdgeKind::Import)]);
    fwd.insert(n("/b"), vec![(n("/c"), EdgeKind::Import)]);
    fwd.insert(n("/c"), vec![]);

    let entries = bfs(&[n("/a")], &fwd, None, None);
    let paths: Vec<_> = entries.iter().map(|e| e.node.as_file().unwrap()).collect();
    assert_eq!(paths, [p("/b").as_path(), p("/c").as_path()]);
    assert_eq!(entries[0].depth, 1);
    assert_eq!(entries[1].depth, 2);
    assert_eq!(entries[0].via, vec![EdgeKind::Import]);
}

#[test]
fn bfs_depth_limit() {
    let mut fwd: EdgeMap = HashMap::new();
    fwd.insert(n("/a"), vec![(n("/b"), EdgeKind::Import)]);
    fwd.insert(n("/b"), vec![(n("/c"), EdgeKind::Import)]);
    fwd.insert(n("/c"), vec![]);

    let entries = bfs(&[n("/a")], &fwd, Some(1), None);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].node.as_file().unwrap(), p("/b").as_path());
}

#[test]
fn bfs_diamond_no_duplicates() {
    let mut fwd: EdgeMap = HashMap::new();
    fwd.insert(
        n("/a"),
        vec![(n("/b"), EdgeKind::Import), (n("/c"), EdgeKind::Import)],
    );
    fwd.insert(n("/b"), vec![(n("/d"), EdgeKind::Import)]);
    fwd.insert(n("/c"), vec![(n("/d"), EdgeKind::Import)]);
    fwd.insert(n("/d"), vec![]);

    let entries = bfs(&[n("/a")], &fwd, None, None);
    let paths: Vec<_> = entries.iter().map(|e| e.node.as_file().unwrap()).collect();
    let unique: HashSet<_> = paths.iter().collect();
    assert_eq!(paths.len(), unique.len(), "no duplicates");
    assert!(entries.iter().any(|e| e.node == n("/d")));
}

#[test]
fn bfs_multiple_roots() {
    let mut fwd: EdgeMap = HashMap::new();
    fwd.insert(n("/a"), vec![(n("/c"), EdgeKind::Import)]);
    fwd.insert(n("/b"), vec![(n("/d"), EdgeKind::Import)]);
    fwd.insert(n("/c"), vec![]);
    fwd.insert(n("/d"), vec![]);

    let entries = bfs(&[n("/a"), n("/b")], &fwd, None, None);
    assert_eq!(entries.len(), 2);
}

#[test]
fn bfs_cycle_terminates() {
    let mut fwd: EdgeMap = HashMap::new();
    fwd.insert(n("/a"), vec![(n("/b"), EdgeKind::Import)]);
    fwd.insert(n("/b"), vec![(n("/a"), EdgeKind::Import)]);

    let entries = bfs(&[n("/a")], &fwd, None, None);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].node.as_file().unwrap(), p("/b").as_path());
}

#[test]
fn bfs_empty_starts() {
    let fwd: EdgeMap = HashMap::new();
    let entries = bfs(&[], &fwd, None, None);
    assert!(entries.is_empty());
}

#[test]
fn bfs_node_with_no_edges() {
    let mut fwd: EdgeMap = HashMap::new();
    fwd.insert(n("/a"), vec![]);
    let entries = bfs(&[n("/a")], &fwd, None, None);
    assert!(entries.is_empty());
}

#[test]
fn bfs_relationship_filter_excludes_wrong_kind() {
    let mut fwd: EdgeMap = HashMap::new();
    fwd.insert(
        n("/a"),
        vec![(n("/b"), EdgeKind::Import), (n("/c"), EdgeKind::TestOf)],
    );
    fwd.insert(n("/b"), vec![]);
    fwd.insert(n("/c"), vec![]);

    let allowed: HashSet<EdgeKind> = [EdgeKind::Import].into();
    let entries = bfs(&[n("/a")], &fwd, None, Some(&allowed));
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].node.as_file().unwrap(), p("/b").as_path());
}

#[test]
fn bfs_via_accumulated_from_two_paths() {
    // a → b via Import; a → b via TestOf (same destination, different kinds)
    let mut fwd: EdgeMap = HashMap::new();
    fwd.insert(
        n("/a"),
        vec![(n("/b"), EdgeKind::Import), (n("/b"), EdgeKind::TestOf)],
    );
    fwd.insert(n("/b"), vec![]);

    let entries = bfs(&[n("/a")], &fwd, None, None);
    assert_eq!(entries.len(), 1);
    // via should contain both kinds
    assert!(entries[0].via.contains(&EdgeKind::Import));
    assert!(entries[0].via.contains(&EdgeKind::TestOf));
}

// ── DepGraph::from_raw_maps ──────────────────────────────────────────────

#[test]
fn dep_graph_deps_of() {
    let fwd = raw_fwd(&[("/root/a.mts", &["/root/b.mts"]), ("/root/b.mts", &[])]);
    let rev = raw_rev(&[]);
    let g = DepGraph::from_raw_maps(p("/root"), fwd, rev);
    let entries = g.deps_of(&[NodeId::File(p("/root/a.mts"))], None, None);
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].node.as_file().unwrap(),
        p("/root/b.mts").as_path()
    );
}

#[test]
fn dep_graph_dependents_of() {
    let fwd = raw_fwd(&[]);
    let rev = raw_rev(&[("/root/b.mts", &["/root/a.mts"])]);
    let g = DepGraph::from_raw_maps(p("/root"), fwd, rev);
    let entries = g.dependents_of(&[NodeId::File(p("/root/b.mts"))], None, None);
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].node.as_file().unwrap(),
        p("/root/a.mts").as_path()
    );
}

// ── DepGraph::build integration ─────────────────────────────────────────

#[test]
fn build_graph_from_fixture() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis")
        .join("simple");
    let root = crate::codebase::ts_resolver::normalize_path(&root);
    let tsconfig = TsConfig {
        dir: root.clone(),
        paths: vec![],
        paths_dir: root.clone(),
    };
    let graph = DepGraph::build(&root, &tsconfig).unwrap();

    let a = root.join("a.mts");
    let b = root.join("b.mts");
    let c = root.join("c.mts");

    let deps_a = graph.deps_of(&[NodeId::File(a.clone())], None, None);
    let dep_paths: Vec<_> = deps_a.iter().filter_map(|e| e.node.as_file()).collect();
    assert!(dep_paths.contains(&b.as_path()));
    assert!(dep_paths.contains(&c.as_path()));

    let dependents_c = graph.dependents_of(&[NodeId::File(c.clone())], None, None);
    let dep_paths: Vec<_> = dependents_c
        .iter()
        .filter_map(|e| e.node.as_file())
        .collect();
    assert!(dep_paths.contains(&b.as_path()));
    assert!(dep_paths.contains(&a.as_path()));
}

#[test]
fn build_graph_aliased_fixture() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis")
        .join("aliased");
    let root = crate::codebase::ts_resolver::normalize_path(&root);
    let tsconfig_path = root.join("tsconfig.json");
    let tsconfig = crate::codebase::ts_resolver::load_tsconfig(&tsconfig_path).unwrap();
    let graph = DepGraph::build(&root, &tsconfig).unwrap();

    let main = root.join("main.mts");
    let helpers = root.join("utils").join("helpers.mts");

    let deps = graph.deps_of(&[NodeId::File(main)], None, None);
    let dep_paths: Vec<_> = deps.iter().filter_map(|e| e.node.as_file()).collect();
    assert!(dep_paths.contains(&helpers.as_path()));
}

#[test]
fn ci_edges_include_workspace_member_bins() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis")
        .join("cargo-workspace-ci");
    let root = crate::codebase::ts_resolver::normalize_path(&root);
    let tsconfig = TsConfig {
        dir: root.clone(),
        paths: vec![],
        paths_dir: root.clone(),
    };
    let graph = DepGraph::build(&root, &tsconfig).unwrap();

    let workflow = root.join(".github").join("workflows").join("ci.yml");
    let implicit_main = root
        .join("crates")
        .join("tool-one")
        .join("src")
        .join("main.rs");
    let hyphenated_bin = root
        .join("crates")
        .join("pg-schema")
        .join("src")
        .join("bin")
        .join("pg-schema.rs");
    let deps = graph.deps_of(
        &[NodeId::File(workflow)],
        None,
        Some(&[EdgeKind::CiInvocation].into()),
    );
    assert!(
        deps.iter()
            .any(|e| e.node.as_file() == Some(implicit_main.as_path())),
        "cargo run -p should link to the member's implicit src/main.rs"
    );
    assert!(
        deps.iter()
            .any(|e| e.node.as_file() == Some(hyphenated_bin.as_path())),
        "cargo run --bin should link to a hyphenated default bin path"
    );
}

#[test]
fn build_graph_excludes_gitignored_files() {
    let dir = TempDir::new().unwrap();
    git_init(dir.path());
    write(dir.path(), ".gitignore", "ignored/\n");
    let source = write(dir.path(), "src/source.mts", "export const value = 1;\n");
    let visible = write(
        dir.path(),
        "src/visible.mts",
        "import { value } from './source.mts';\n",
    );
    write(
        dir.path(),
        "ignored/hidden.mts",
        "import { value } from '../src/source.mts';\n",
    );
    git_add_all(dir.path());

    let tsconfig = TsConfig {
        dir: dir.path().to_path_buf(),
        paths: vec![],
        paths_dir: dir.path().to_path_buf(),
    };
    let graph = DepGraph::build(dir.path(), &tsconfig).unwrap();

    let dependents = graph.dependents_of(&[NodeId::File(source)], None, None);
    let paths: Vec<_> = dependents.iter().filter_map(|e| e.node.as_file()).collect();
    assert_eq!(paths, vec![visible.as_path()]);
}

#[test]
fn graph_build_plan_import_only_enables_only_imports() {
    let allowed: HashSet<EdgeKind> = [EdgeKind::Import, EdgeKind::TypeImport].into();

    let plan = GraphBuildPlan::from_allowed(Some(&allowed));

    assert!(plan.import_only());
}

#[test]
fn package_dependency_names_returns_dependency_names() {
    let package_json = serde_json::json!({
        "dependencies": {
            "@scope/local": "workspace:^",
            "external": "^1.0.0"
        },
        "devDependencies": {
            "@scope/dev-local": "workspace:*"
        }
    });

    let names = package_dependency_names(&package_json);

    assert_eq!(names, vec!["@scope/dev-local", "@scope/local", "external"]);
}

#[test]
fn lazy_import_deps_walks_only_reachable_import_graph() {
    let dir = TempDir::new().unwrap();
    git_init(dir.path());
    let entry = write(dir.path(), "src/a.mts", "import './b.mts';\n");
    let b = write(dir.path(), "src/b.mts", "export const b = 1;\n");
    write(
        dir.path(),
        "src/unrelated.mts",
        "import './unrelated-dep.mts';\n",
    );
    write(
        dir.path(),
        "src/unrelated-dep.mts",
        "export const other = 1;\n",
    );
    git_add_all(dir.path());

    let tsconfig = TsConfig {
        dir: dir.path().to_path_buf(),
        paths: vec![],
        paths_dir: dir.path().to_path_buf(),
    };

    let deps = lazy_import_deps_of(&[NodeId::File(entry)], dir.path(), &tsconfig, None).unwrap();

    assert_eq!(
        deps.iter()
            .filter_map(|entry| entry.node.as_file())
            .collect::<Vec<_>>(),
        vec![b.as_path()]
    );
}

// ── build_filter / apply_filter ─────────────────────────────────────────

#[test]
fn build_filter_none_for_empty() {
    let f = build_filter(&[]).unwrap();
    assert!(f.is_none());
}

#[test]
fn build_filter_matches_glob() {
    let spec = build_filter(&["**/*.test.mts".to_string()])
        .unwrap()
        .unwrap();
    let root = p("/root");
    let entries = vec![
        mk_entry("/root/src/foo.test.mts", 1),
        mk_entry("/root/src/foo.mts", 1),
    ];
    let result = apply_filter(entries, Some(&spec), &root);
    assert_eq!(result.len(), 1);
    assert!(result[0]
        .node
        .as_file()
        .unwrap()
        .to_str()
        .unwrap()
        .contains("foo.test.mts"));
}

// ── add_test_edges direction ─────────────────────────────────────────────

#[test]
fn test_of_edges_do_not_make_source_depend_on_test() {
    // Regression: previously add_test_edges emitted forward[src→test] which
    // made `dependencies foo.mts` return its test file as a forward dep.
    let src = p("/root/foo.mts");
    let test = p("/root/foo.test.mts");
    let mut forward: EdgeMap = HashMap::new();
    let mut reverse: EdgeMap = HashMap::new();
    merge_edges(
        &mut forward,
        &mut reverse,
        collect_test_edges(&[src.clone(), test.clone()]),
    );

    // forward: test→src only (test depends on source)
    let test_fwd: Vec<_> = forward
        .get(&NodeId::File(test.clone()))
        .unwrap_or(&vec![])
        .iter()
        .map(|(n, _)| n.clone())
        .collect();
    assert!(
        test_fwd.contains(&NodeId::File(src.clone())),
        "forward test→src"
    );
    let src_fwd: Vec<_> = forward
        .get(&NodeId::File(src.clone()))
        .unwrap_or(&vec![])
        .iter()
        .map(|(n, _)| n.clone())
        .collect();
    assert!(
        !src_fwd.contains(&NodeId::File(test.clone())),
        "forward src→test must NOT exist"
    );

    // reverse: src→test only (source is tested by test file)
    let src_rev: Vec<_> = reverse
        .get(&NodeId::File(src.clone()))
        .unwrap_or(&vec![])
        .iter()
        .map(|(n, _)| n.clone())
        .collect();
    assert!(
        src_rev.contains(&NodeId::File(test.clone())),
        "reverse src→test"
    );
    let test_rev: Vec<_> = reverse
        .get(&NodeId::File(test.clone()))
        .unwrap_or(&vec![])
        .iter()
        .map(|(n, _)| n.clone())
        .collect();
    assert!(
        !test_rev.contains(&NodeId::File(src.clone())),
        "reverse test→src must NOT exist"
    );
}

#[test]
fn apply_filter_none_keeps_all() {
    let entries = vec![mk_entry("/a.ts", 1), mk_entry("/b.ts", 2)];
    let result = apply_filter(entries.clone(), None, p("/").as_path());
    assert_eq!(result.len(), 2);
}

#[test]
fn apply_filter_removes_non_matching() {
    let spec = build_filter(&["**/*.test.ts".to_string()])
        .unwrap()
        .unwrap();
    let root = p("/root");
    let entries = vec![
        mk_entry("/root/src/foo.test.ts", 1),
        mk_entry("/root/src/foo.ts", 1),
    ];
    let result = apply_filter(entries, Some(&spec), &root);
    assert_eq!(result.len(), 1);
    assert!(result[0]
        .node
        .as_file()
        .unwrap()
        .to_str()
        .unwrap()
        .contains(".test.ts"));
}

#[test]
fn apply_filter_passes_queue_job_nodes() {
    let spec = build_filter(&["**/*.test.ts".to_string()])
        .unwrap()
        .unwrap();
    let root = p("/root");
    let queue_job = NodeEntry {
        node: NodeId::QueueJob {
            queue_file: p("/root/src/queues.mts"),
            job: "sendWelcome".to_string(),
        },
        depth: 1,
        via: vec![],
    };
    let file_entry = mk_entry("/root/src/foo.mts", 1);
    let entries = vec![queue_job, file_entry];
    let result = apply_filter(entries, Some(&spec), &root);
    // QueueJob node passes through (not path-filtered); file doesn't match
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].node, NodeId::QueueJob { .. }));
}

// ── folder-suffix filter ─────────────────────────────────────────────────

#[test]
fn folder_suffix_collapses_to_folder() {
    let spec = build_filter(&["backend/systems/*/".to_string()])
        .unwrap()
        .unwrap();
    let root = p("/project");
    let entries = vec![
        mk_entry("/project/backend/systems/emails/index.mts", 1),
        mk_entry("/project/backend/systems/emails/helpers.mts", 2),
        mk_entry("/project/backend/systems/users/index.mts", 1),
    ];
    let result = apply_filter(entries, Some(&spec), &root);
    assert_eq!(result.len(), 2);
    let paths: Vec<_> = result
        .iter()
        .map(|e| e.node.as_file().unwrap().to_str().unwrap())
        .collect();
    assert!(paths.iter().any(|p| p.ends_with("emails")));
    assert!(paths.iter().any(|p| p.ends_with("users")));
}

#[test]
fn folder_suffix_uses_min_depth() {
    let spec = build_filter(&["systems/*/".to_string()]).unwrap().unwrap();
    let root = p("/root");
    let entries = vec![
        mk_entry("/root/systems/emails/deep/file.mts", 3),
        mk_entry("/root/systems/emails/shallow.mts", 1),
    ];
    let result = apply_filter(entries, Some(&spec), &root);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].depth, 1);
}

#[test]
fn folder_suffix_and_file_glob_combined() {
    let spec = build_filter(&["systems/*/".to_string(), "**/*.test.mts".to_string()])
        .unwrap()
        .unwrap();
    let root = p("/root");
    let entries = vec![
        mk_entry("/root/systems/emails/a.mts", 1),
        mk_entry("/root/other/foo.test.mts", 2),
        mk_entry("/root/other/foo.mts", 2),
    ];
    let result = apply_filter(entries, Some(&spec), &root);
    assert_eq!(result.len(), 2);
}

#[test]
fn folder_suffix_empty_produces_no_entries() {
    let spec = build_filter(&["nomatch/*/".to_string()]).unwrap().unwrap();
    let root = p("/root");
    let entries = vec![mk_entry("/root/other/file.mts", 1)];
    let result = apply_filter(entries, Some(&spec), &root);
    assert!(result.is_empty());
}

// ── SymbolIndex ──────────────────────────────────────────────────────────

#[test]
fn symbol_index_basic_lookup() {
    let mut map: HashMap<PathBuf, Vec<(PathBuf, String, String, bool)>> = HashMap::new();
    map.insert(
        p("/src/b.mts"),
        vec![(
            p("/src/a.mts"),
            "alpha".to_string(),
            "alpha".to_string(),
            false,
        )],
    );
    let index = SymbolIndex::build(&map);
    let importers = index
        .importers_of(p("/src/a.mts").as_path(), "alpha")
        .unwrap();
    assert_eq!(importers.len(), 1);
    assert_eq!(importers[0].0, p("/src/b.mts"));
}

#[test]
fn symbol_index_missing_returns_none() {
    let map: HashMap<PathBuf, Vec<(PathBuf, String, String, bool)>> = HashMap::new();
    let index = SymbolIndex::build(&map);
    assert!(index
        .importers_of(p("/src/a.mts").as_path(), "ghost")
        .is_none());
}

#[test]
fn symbol_index_multiple_importers() {
    let mut map: HashMap<PathBuf, Vec<(PathBuf, String, String, bool)>> = HashMap::new();
    map.insert(
        p("/b.mts"),
        vec![(p("/a.mts"), "fn1".to_string(), "fn1".to_string(), false)],
    );
    map.insert(
        p("/c.mts"),
        vec![(p("/a.mts"), "fn1".to_string(), "fn1".to_string(), false)],
    );
    let index = SymbolIndex::build(&map);
    let importers = index.importers_of(p("/a.mts").as_path(), "fn1").unwrap();
    assert_eq!(importers.len(), 2);
}

// ── add_test_edges ───────────────────────────────────────────────────────

#[test]
fn test_edges_source_finds_test_file() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis")
        .join("test-framework")
        .join("src");
    let root = crate::codebase::ts_resolver::normalize_path(&root);
    let tsconfig = TsConfig {
        dir: root.clone(),
        paths: vec![],
        paths_dir: root.clone(),
    };
    let graph = DepGraph::build(&root, &tsconfig).unwrap();

    let index_mts = root.join("index.mts");
    let index_test = root.join("index.test.mts");
    let testof_filter: HashSet<EdgeKind> = [EdgeKind::TestOf].into();

    // dependents_of (reverse walk): test file is a dependent of its source.
    let dependents = graph.dependents_of(
        &[NodeId::File(index_mts.clone())],
        None,
        Some(&testof_filter),
    );
    assert!(
        dependents
            .iter()
            .any(|e| e.node.as_file() == Some(index_test.as_path())),
        "index.test.mts should appear as a dependent of index.mts"
    );

    // deps_of (forward walk): source file must NOT forward-depend on its test.
    let deps = graph.deps_of(&[NodeId::File(index_mts)], None, Some(&testof_filter));
    assert!(
        !deps
            .iter()
            .any(|e| e.node.as_file() == Some(index_test.as_path())),
        "index.mts must NOT forward-depend on index.test.mts"
    );
}

// ── add_md_edges ─────────────────────────────────────────────────────────

#[test]
fn md_edges_added_for_codebase_intel_fixture() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis")
        .join("codebase-intel");
    let root = crate::codebase::ts_resolver::normalize_path(&root);
    let tsconfig = TsConfig {
        dir: root.clone(),
        paths: vec![],
        paths_dir: root.clone(),
    };
    let graph = DepGraph::build(&root, &tsconfig).unwrap();

    let readme = root.join("README.md");
    let deps = graph.deps_of(
        &[NodeId::File(readme)],
        None,
        Some(&[EdgeKind::MarkdownLink].into()),
    );
    // README.md links to packages/api/src/index.mts
    let linked_file = root
        .join("packages")
        .join("api")
        .join("src")
        .join("index.mts");
    assert!(
        deps.iter()
            .any(|e| e.node.as_file() == Some(linked_file.as_path())),
        "README.md should have MarkdownLink edge to packages/api/src/index.mts"
    );
}

// ── package_name_from_spec ───────────────────────────────────────────────

#[test]
fn pkg_name_scoped_no_subpath() {
    assert_eq!(package_name_from_spec("@x/api"), "@x/api");
}

#[test]
fn pkg_name_scoped_with_subpath() {
    assert_eq!(package_name_from_spec("@x/api/utils"), "@x/api");
}

#[test]
fn pkg_name_unscoped_no_subpath() {
    assert_eq!(package_name_from_spec("lodash"), "lodash");
}

#[test]
fn pkg_name_unscoped_with_subpath() {
    assert_eq!(package_name_from_spec("lodash/merge"), "lodash");
}
