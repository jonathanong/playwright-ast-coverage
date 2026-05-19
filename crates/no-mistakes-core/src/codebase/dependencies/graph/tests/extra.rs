use super::*;

fn codebase_intel() -> (PathBuf, TsConfig, DepGraph) {
    let root = crate::codebase::ts_resolver::normalize_path(&fixture("codebase-intel"));
    let tsconfig =
        crate::codebase::ts_resolver::load_tsconfig(&root.join("tsconfig.json")).unwrap();
    let graph = build_graph(&root, &tsconfig);
    (root, tsconfig, graph)
}

fn has_file(entries: &[NodeEntry], path: &Path) -> bool {
    entries
        .iter()
        .any(|entry| entry.node.as_file() == Some(path))
}

#[test]
fn lazy_import_handles_depth_virtual_roots_hidden_targets_and_duplicate_kinds() {
    let root = crate::codebase::ts_resolver::normalize_path(&fixture("simple"));
    let tsconfig = TsConfig {
        dir: root.clone(),
        paths: vec![],
        paths_dir: root.clone(),
        base_url: None,
    };
    let a = root.join("a.mts");
    let b = root.join("b.mts");
    let c = root.join("c.mts");
    let hidden = root.join("hidden.mts");
    let graph_files = GraphFiles {
        all: vec![a.clone(), b.clone(), c.clone(), hidden.clone()],
        indexable: vec![a.clone(), b.clone(), c.clone(), hidden],
        visible: [a.clone(), b.clone(), c.clone()].into(),
    };

    let roots = vec![
        NodeId::QueueJob {
            queue_file: a.clone(),
            job: "send".to_string(),
        },
        NodeId::File(a),
    ];
    let limited = lazy_import_deps_of_with_files(&roots, &root, &tsconfig, Some(1), &graph_files);
    assert!(has_file(&limited, &b));
    assert!(!has_file(&limited, &c));

    let full = lazy_import_deps_of_with_files(
        &[NodeId::File(root.join("a.mts"))],
        &root,
        &tsconfig,
        None,
        &graph_files,
    );
    assert!(has_file(&full, &b));
    assert!(has_file(&full, &c));

    let duplicate_root = crate::codebase::ts_resolver::normalize_path(&fixture("lazy-duplicates"));
    let duplicate_tsconfig = TsConfig {
        dir: duplicate_root.clone(),
        paths: vec![],
        paths_dir: duplicate_root.clone(),
        base_url: None,
    };
    let duplicate_files = GraphFiles::discover(&duplicate_root);
    let duplicate = lazy_import_deps_of_with_files(
        &[NodeId::File(duplicate_root.join("a.mts"))],
        &duplicate_root,
        &duplicate_tsconfig,
        None,
        &duplicate_files,
    );
    let duplicate_b = duplicate_root.join("b.mts");
    let b_entry = duplicate
        .iter()
        .find(|entry| entry.node.as_file() == Some(duplicate_b.as_path()))
        .unwrap();
    assert_eq!(b_entry.via, vec![EdgeKind::Import, EdgeKind::TypeImport]);

    let hidden_root = crate::codebase::ts_resolver::normalize_path(&fixture("lazy-hidden"));
    let hidden_tsconfig = TsConfig {
        dir: hidden_root.clone(),
        paths: vec![],
        paths_dir: hidden_root.clone(),
        base_url: None,
    };
    let hidden_graph_files = GraphFiles {
        all: vec![hidden_root.join("a.mts"), hidden_root.join("hidden.mts")],
        indexable: vec![hidden_root.join("a.mts"), hidden_root.join("hidden.mts")],
        visible: [hidden_root.join("a.mts")].into(),
    };
    assert!(lazy_import_deps_of_with_files(
        &[NodeId::File(hidden_root.join("a.mts"))],
        &hidden_root,
        &hidden_tsconfig,
        None,
        &hidden_graph_files,
    )
    .is_empty());
}

#[test]
fn low_level_collectors_cover_empty_invalid_and_non_visible_branches() {
    let root = crate::codebase::ts_resolver::normalize_path(&fixture("codebase-intel"));
    let tsconfig =
        crate::codebase::ts_resolver::load_tsconfig(&root.join("tsconfig.json")).unwrap();
    let resolver = crate::codebase::ts_resolver::ImportResolver::new(&tsconfig);
    let package = root.join("package.json");
    let web_entry = root.join("packages/web/src/index.tsx");
    let hidden = root.join("packages/api/src/index.mts");
    let graph_files = GraphFiles {
        all: vec![package.clone(), web_entry.clone(), hidden],
        indexable: vec![web_entry.clone()],
        visible: [package.clone(), web_entry.clone()].into(),
    };
    let workspace = crate::codebase::workspaces::WorkspaceMap {
        packages: vec![
            crate::codebase::workspaces::WorkspacePackage {
                name: "@x/web".to_string(),
                dir: root.join("packages/web"),
                entry: Some(web_entry.clone()),
                exports: None,
            },
            crate::codebase::workspaces::WorkspacePackage {
                name: "@x/hidden".to_string(),
                dir: root.join("hidden"),
                entry: Some(root.join("hidden/index.ts")),
                exports: None,
            },
        ],
    };

    assert!(
        collect_workspace_edges(&vec![], &resolver, &Default::default(), &graph_files).is_empty()
    );
    let imports = vec![(
        root.join("packages/api/src/users.mts"),
        vec![ExtractedImport {
            specifier: "@x/web".to_string(),
            kind: ImportKind::Static,
        }],
    )];
    let edges = collect_workspace_edges(&imports, &resolver, &workspace, &graph_files);
    assert_eq!(edges.len(), 1);
    let hidden_workspace_import = vec![(
        root.join("packages/api/src/users.mts"),
        vec![ExtractedImport {
            specifier: "@x/hidden".to_string(),
            kind: ImportKind::Static,
        }],
    )];
    assert!(collect_workspace_edges(
        &hidden_workspace_import,
        &resolver,
        &workspace,
        &graph_files
    )
    .is_empty());

    let manifest_edges = collect_workspace_manifest_edges(
        &[
            package.clone(),
            root.join("missing/package.json"),
            root.join("bad/package.json"),
            root.join("hidden/package.json"),
        ],
        &workspace,
        &graph_files,
    );
    assert!(manifest_edges.iter().any(|(_, to, kind)| {
        *kind == EdgeKind::WorkspaceImport && to.as_file() == Some(web_entry.as_path())
    }));

    assert_eq!(
        collect_import_edges(&imports, &resolver, &graph_files).len(),
        1
    );
    let hidden_imports = vec![(
        root.join("packages/api/src/users.mts"),
        vec![ExtractedImport {
            specifier: "./index.mts".to_string(),
            kind: ImportKind::Static,
        }],
    )];
    assert!(collect_import_edges(&hidden_imports, &resolver, &graph_files).is_empty());
    assert_eq!(package_name_from_spec("@scope/pkg/path"), "@scope/pkg");
    assert_eq!(package_name_from_spec("@scope"), "@scope");
}

#[test]
fn graph_helpers_cover_test_markdown_ci_symbol_and_queue_paths() {
    let (root, tsconfig, graph) = codebase_intel();
    let emails = root.join("packages/api/src/emails.mts");
    let send_email = root.join("packages/api/src/send-email.mts");

    assert_eq!(graph.root(), root.as_path());
    assert!(graph
        .all_files()
        .any(|node| node.as_file() == Some(emails.as_path())));

    let symbol_index = SymbolIndex::build_from_root(&root, &tsconfig).unwrap();
    let dependents = graph.dependents_of_symbol(
        &emails,
        "sendWelcomeEmail",
        None,
        Some(&[EdgeKind::QueueEnqueue].into()),
        &symbol_index,
    );
    assert!(dependents.iter().any(|entry| {
        matches!(
            entry.node,
            NodeId::File(ref path) if path == &send_email
        )
    }));

    let missing_root = root.join("does-not-exist");
    let missing_tsconfig = TsConfig {
        dir: missing_root.clone(),
        paths: vec![],
        paths_dir: missing_root.clone(),
        base_url: None,
    };
    assert!(
        SymbolIndex::build_from_root(&missing_root, &missing_tsconfig)
            .unwrap()
            .importers_of(&emails, "none")
            .is_none()
    );

    let graph_files = GraphFiles::discover(&root);
    let missing = root.join("missing.md");
    assert!(collect_md_edges(&[missing], &graph_files).is_empty());
    let md_edges = collect_md_edges(&[root.join("README.md")], &graph_files);
    assert!(md_edges.iter().any(|(_, to, kind)| {
        *kind == EdgeKind::MarkdownLink
            && to.as_file() == Some(root.join("packages/api/src/index.mts").as_path())
    }));

    let mut forward = EdgeMap::new();
    let mut reverse = EdgeMap::new();
    add_ci_edges(&root, &graph_files.all, &mut forward, &mut reverse);
    assert!(!forward.is_empty());

    let mut missing_forward = EdgeMap::new();
    let mut missing_reverse = EdgeMap::new();
    let missing_workflow = root.join(".github/workflows/missing.yml");
    add_ci_edges(
        &root,
        &[
            root.join("Cargo.toml"),
            root.join("src/bin/guardrails.rs"),
            missing_workflow,
            root.join(".github/workflows/not-yaml.txt"),
        ],
        &mut missing_forward,
        &mut missing_reverse,
    );
    assert!(missing_forward.is_empty());

    let nested_root = crate::codebase::ts_resolver::normalize_path(&fixture("cargo-nested-bin"));
    assert_eq!(
        resolve_cargo_bin_source(&nested_root, "nested", "missing.rs"),
        Some(nested_root.join("src/bin/nested/main.rs"))
    );
    let mut nested_forward = EdgeMap::new();
    let mut nested_reverse = EdgeMap::new();
    add_ci_edges(
        &nested_root,
        &[
            nested_root.join("Cargo.toml"),
            nested_root.join("src/bin/nested/main.rs"),
            nested_root.join(".github/workflows/bad.yml"),
        ],
        &mut nested_forward,
        &mut nested_reverse,
    );
    assert!(nested_forward.is_empty());
    add_ci_edges(
        &nested_root,
        &[
            nested_root.join("Cargo.toml"),
            nested_root.join("src/bin/nested/main.rs"),
        ],
        &mut nested_forward,
        &mut nested_reverse,
    );
    assert!(nested_forward.is_empty());
    let mut bins = CargoBinIndex::default();
    add_manifest_bins(
        Path::new("/"),
        "[[bin]]\nname = \"root\"\npath = \"main.rs\"\n",
        &mut bins,
    );
    add_manifest_bins(&root.join("Cargo.toml"), "[[bin]", &mut bins);
    let outside = collect_cargo_bins(&root, &[PathBuf::from("/outside/Cargo.toml")]);
    assert!(outside.by_name.contains_key("guardrails"));
    let missing_member_manifest = collect_cargo_bins(&root, &[root.join("missing/Cargo.toml")]);
    assert!(missing_member_manifest.by_name.contains_key("guardrails"));
    let invalid_root = crate::codebase::ts_resolver::normalize_path(&fixture("cargo-invalid"));
    assert!(
        collect_cargo_bins(&invalid_root, &[invalid_root.join("Cargo.toml")])
            .by_name
            .is_empty()
    );
}

#[test]
fn graph_collectors_cover_defensive_empty_and_error_paths() {
    let root = crate::codebase::ts_resolver::normalize_path(&fixture("codebase-intel"));
    let tsconfig =
        crate::codebase::ts_resolver::load_tsconfig(&root.join("tsconfig.json")).unwrap();
    let graph_files = GraphFiles {
        all: vec![],
        indexable: vec![],
        visible: HashSet::new(),
    };

    assert!(lazy_import_deps_of_with_files(
        &[NodeId::File(root.join("packages/api/src/index.mts"))],
        &root,
        &tsconfig,
        None,
        &graph_files,
    )
    .is_empty());
    assert!(import_neighbors(
        &root.join("missing.mts"),
        &crate::codebase::ts_resolver::ImportResolver::new(&tsconfig),
        &ImportExtractor::for_typescript().unwrap(),
        &ImportExtractor::for_tsx().unwrap(),
        &graph_files,
    )
    .is_empty());

    assert!(collect_workspace_manifest_edges(
        &[root.join("missing/package.json")],
        &crate::codebase::workspaces::WorkspaceMap {
            packages: vec![crate::codebase::workspaces::WorkspacePackage {
                name: "@x/missing".to_string(),
                dir: root.join("packages/missing"),
                entry: Some(root.join("packages/missing/index.ts")),
                exports: None,
            }],
        },
        &graph_files,
    )
    .is_empty());
    assert!(collect_test_edges(&[PathBuf::from("/")]).is_empty());
    assert!(collect_test_edges(&[PathBuf::from("no-parent.ts")]).is_empty());
    assert!(collect_md_edges(&[PathBuf::from("/")], &graph_files).is_empty());
    assert!(collect_md_edges(&[PathBuf::from("README.md")], &graph_files).is_empty());

    let mut forward = EdgeMap::new();
    let mut reverse = EdgeMap::new();
    add_ci_edges(&root.join("missing"), &[], &mut forward, &mut reverse);
    assert!(forward.is_empty());

    assert!(collect_route_edges(&root.join("missing"), &tsconfig, &[], None).is_empty());
    add_queue_edges(
        &root.join("missing"),
        &crate::codebase::ts_resolver::ImportResolver::new(&tsconfig),
        &[],
        None,
        &mut forward,
        &mut reverse,
    );
    assert!(collect_http_call_edges(&root.join("missing"), &tsconfig, None, &[], &[]).is_empty());
}

#[test]
fn graph_collectors_cover_malformed_and_invalid_config_branches() {
    let source_root = crate::codebase::ts_resolver::normalize_path(&fixture("codebase-intel"));
    let tsconfig =
        crate::codebase::ts_resolver::load_tsconfig(&source_root.join("tsconfig.json")).unwrap();
    let files = vec![source_root.join("packages/api/src/index.mts")];
    let resolver = crate::codebase::ts_resolver::ImportResolver::new(&tsconfig);

    let malformed =
        crate::codebase::ts_resolver::normalize_path(&fixture("graph-malformed-config"));
    let invalid = crate::codebase::ts_resolver::normalize_path(&fixture("graph-invalid-globs"));
    let empty = crate::codebase::ts_resolver::normalize_path(&fixture("graph-empty-route-config"));
    let frontend_only =
        crate::codebase::ts_resolver::normalize_path(&fixture("playwright-coverage"));
    let frontend_files = GraphFiles::discover(&frontend_only).all;

    assert!(collect_route_edges(&malformed, &tsconfig, &files, None).is_empty());
    assert!(collect_route_edges(&invalid, &tsconfig, &files, None).is_empty());
    assert!(collect_route_edges(&empty, &tsconfig, &files, None).is_empty());
    assert!(collect_route_edges(&frontend_only, &tsconfig, &frontend_files, None).is_empty());

    let mut forward = EdgeMap::new();
    let mut reverse = EdgeMap::new();
    add_queue_edges(
        &malformed,
        &resolver,
        &files,
        None,
        &mut forward,
        &mut reverse,
    );
    add_queue_edges(
        &invalid,
        &resolver,
        &files,
        None,
        &mut forward,
        &mut reverse,
    );
    add_queue_edges(&empty, &resolver, &files, None, &mut forward, &mut reverse);
    assert!(forward.is_empty());

    let sources = vec![(files[0].clone(), "fetch('/api/users')".to_string())];
    assert!(collect_http_call_edges(&malformed, &tsconfig, None, &sources, &files).is_empty());
    assert!(collect_http_call_edges(&invalid, &tsconfig, None, &sources, &files).is_empty());
}

#[test]
fn codebase_intel_graph_emits_queue_http_route_test_and_process_edges() {
    let (root, _tsconfig, graph) = codebase_intel();

    let send_email = root.join("packages/api/src/send-email.mts");
    let emails = root.join("packages/api/src/emails.mts");
    let processors = root.join("packages/api/src/processors.mts");
    let worker = root.join("packages/api/src/worker.mts");
    let api_client = root.join("packages/web/src/api-client.tsx");
    let api_index = root.join("packages/api/src/index.mts");
    let spec = root.join("tests/e2e/users.spec.ts");
    let page = root.join("packages/web/app/users/[id]/page.tsx");
    let spawner = root.join("packages/api/src/spawn-runner.mts");
    let spawn_target = root.join("packages/api/src/spawn-target.mts");

    let enqueue = graph.deps_of(
        &[NodeId::File(send_email)],
        None,
        Some(&[EdgeKind::QueueEnqueue].into()),
    );
    assert!(enqueue.iter().any(|entry| {
        matches!(
            &entry.node,
            NodeId::QueueJob { queue_file, job }
                if queue_file == &emails && job == "sendWelcomeEmail"
        )
    }));

    let queue_job = NodeId::QueueJob {
        queue_file: emails,
        job: "sendWelcomeEmail".to_string(),
    };
    let workers = graph.deps_of(&[queue_job], None, Some(&[EdgeKind::QueueWorker].into()));
    assert!(has_file(&workers, &processors));
    assert!(has_file(&workers, &worker));

    let http = graph.deps_of(
        &[NodeId::File(api_client.clone())],
        None,
        Some(&[EdgeKind::HttpCall].into()),
    );
    assert!(has_file(&http, &api_index));

    let route_refs = graph.deps_of(
        &[NodeId::File(api_client)],
        None,
        Some(&[EdgeKind::RouteRef].into()),
    );
    assert!(has_file(&route_refs, &api_index));

    let route_tests = graph.deps_of(
        &[NodeId::File(spec)],
        None,
        Some(&[EdgeKind::RouteTest].into()),
    );
    assert!(has_file(&route_tests, &page));

    let process = graph.deps_of(
        &[NodeId::File(spawner)],
        None,
        Some(&[EdgeKind::ProcessSpawn].into()),
    );
    assert!(has_file(&process, &spawn_target));
}

#[test]
fn processor_export_kind_accepts_runtime_exports_only() {
    assert!(is_processor_export_kind(&ExportKind::Function));
    assert!(is_processor_export_kind(&ExportKind::Const));
    assert!(is_processor_export_kind(&ExportKind::Let));
    assert!(is_processor_export_kind(&ExportKind::Var));
    assert!(!is_processor_export_kind(&ExportKind::TypeAlias));
    assert!(!is_processor_export_kind(&ExportKind::Interface));
    assert!(!is_processor_export_kind(&ExportKind::Default));
}

#[test]
fn route_collectors_cover_configured_prefixes_and_scan_globs() {
    let root = crate::codebase::ts_resolver::normalize_path(&fixture("graph-default-route-config"));
    let tsconfig =
        crate::codebase::ts_resolver::load_tsconfig(&root.join("tsconfig.json")).unwrap();
    let all_files = GraphFiles::discover(&root).all;
    let client = root.join("src/client.ts");
    let route = root.join("backend/api/users.mts");

    let route_edges = collect_route_edges(&root, &tsconfig, &all_files, None);
    assert!(route_edges.iter().any(|(from, to, kind)| {
        *kind == EdgeKind::RouteRef
            && from.as_file() == Some(client.as_path())
            && to.as_file() == Some(route.as_path())
    }));

    let sources = vec![(client.clone(), std::fs::read_to_string(&client).unwrap())];
    let http_edges = collect_http_call_edges(&root, &tsconfig, None, &sources, &all_files);
    assert!(http_edges.iter().any(|(from, to, kind)| {
        *kind == EdgeKind::HttpCall
            && from.as_file() == Some(client.as_path())
            && to.as_file() == Some(route.as_path())
    }));
}
