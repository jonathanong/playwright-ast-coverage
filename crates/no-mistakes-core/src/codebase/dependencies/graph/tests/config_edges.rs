use super::*;

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
    let malformed_options = graph_config_options(&malformed);
    let invalid_options = graph_config_options(&invalid);
    let empty_options = graph_config_options(&empty);
    let frontend_options = graph_config_options(&frontend_only);

    assert!(collect_route_edges(
        &malformed,
        &tsconfig,
        &files,
        None,
        malformed_options.as_ref()
    )
    .is_empty());
    assert!(
        collect_route_edges(&invalid, &tsconfig, &files, None, invalid_options.as_ref()).is_empty()
    );
    assert!(
        collect_route_edges(&empty, &tsconfig, &files, None, empty_options.as_ref()).is_empty()
    );
    assert!(collect_route_edges(
        &frontend_only,
        &tsconfig,
        &frontend_files,
        None,
        frontend_options.as_ref(),
    )
    .is_empty());

    let mut forward = EdgeMap::new();
    let mut reverse = EdgeMap::new();
    add_queue_edges(
        &malformed,
        &resolver,
        &files,
        None,
        malformed_options.as_ref(),
        &mut forward,
        &mut reverse,
    );
    add_queue_edges(
        &invalid,
        &resolver,
        &files,
        None,
        invalid_options.as_ref(),
        &mut forward,
        &mut reverse,
    );
    add_queue_edges(
        &empty,
        &resolver,
        &files,
        None,
        empty_options.as_ref(),
        &mut forward,
        &mut reverse,
    );
    assert!(forward.is_empty());

    let sources = vec![(files[0].clone(), "fetch('/api/users')".to_string())];
    assert!(collect_http_call_edges(
        &malformed,
        &tsconfig,
        None,
        &sources,
        &files,
        &files,
        malformed_options.as_ref(),
    )
    .is_empty());
    assert!(collect_http_call_edges(
        &invalid,
        &tsconfig,
        None,
        &sources,
        &files,
        &files,
        invalid_options.as_ref(),
    )
    .is_empty());
}

#[test]
fn route_collectors_cover_configured_prefixes_and_scan_globs() {
    let root = crate::codebase::ts_resolver::normalize_path(&fixture("graph-default-route-config"));
    let tsconfig =
        crate::codebase::ts_resolver::load_tsconfig(&root.join("tsconfig.json")).unwrap();
    let all_files = GraphFiles::discover(&root).all;
    let client = root.join("src/client.ts");
    let route = root.join("backend/api/users.mts");
    let fake_route = root.join("src/fake-backend.mts");
    let config_options = graph_config_options(&root);

    let route_edges =
        collect_route_edges(&root, &tsconfig, &all_files, None, config_options.as_ref());
    assert!(route_edges.iter().any(|(from, to, kind)| {
        *kind == EdgeKind::RouteRef
            && from.as_file() == Some(client.as_path())
            && to.as_file() == Some(route.as_path())
    }));

    let sources = vec![(client.clone(), std::fs::read_to_string(&client).unwrap())];
    let http_edges = collect_http_call_edges(
        &root,
        &tsconfig,
        None,
        &sources,
        &all_files,
        &all_files,
        config_options.as_ref(),
    );
    assert!(http_edges.iter().any(|(from, to, kind)| {
        *kind == EdgeKind::HttpCall
            && from.as_file() == Some(client.as_path())
            && to.as_file() == Some(route.as_path())
    }));

    let fact_plan = GraphBuildPlan {
        routes: true,
        http: true,
        ..GraphBuildPlan::default()
    };
    let fact_context = ts_fact_context_for_plan(&root, fact_plan);
    let facts = collect_ts_facts_with_context(&all_files, fact_plan.ts_fact_plan(), &fact_context);
    assert!(facts
        .get(&fake_route)
        .expect("fake route source should be parsed")
        .backend_routes
        .is_empty());

    let http_edges_with_facts = collect_http_call_edges(
        &root,
        &tsconfig,
        Some(&facts),
        &[],
        &all_files,
        &all_files,
        config_options.as_ref(),
    );
    assert!(http_edges_with_facts.iter().any(|(from, to, kind)| {
        *kind == EdgeKind::HttpCall
            && from.as_file() == Some(client.as_path())
            && to.as_file() == Some(route.as_path())
    }));
}

#[test]
fn route_and_http_fact_context_keep_separate_backend_matchers() {
    let root =
        crate::codebase::ts_resolver::normalize_path(&fixture("graph-split-route-http-config"));
    let tsconfig =
        crate::codebase::ts_resolver::load_tsconfig(&root.join("tsconfig.json")).unwrap();
    let all_files = GraphFiles::discover(&root).all;
    let client = root.join("src/client.ts");
    let route_def = root.join("routes/users.mts");
    let http_def = root.join("http/users.mts");
    let config_options = graph_config_options(&root);
    let plan = GraphBuildPlan {
        routes: true,
        http: true,
        ..GraphBuildPlan::default()
    };
    let context = ts_fact_context_for_plan(&root, plan);
    assert_eq!(context.backend_route_extractors.len(), 2);

    let facts = collect_ts_facts_with_context(&all_files, plan.ts_fact_plan(), &context);
    assert!(facts[&route_def]
        .backend_routes
        .iter()
        .any(|route| { route.register_object == "routeApp" && route.route == "/route/users/:id" }));
    assert!(facts[&http_def]
        .backend_routes
        .iter()
        .any(|route| { route.register_object == "httpApp" && route.route == "/http/users/:id" }));
    assert!(facts[&route_def]
        .backend_routes
        .iter()
        .all(|route| route.register_object != "httpApp"));
    assert!(facts[&http_def]
        .backend_routes
        .iter()
        .all(|route| route.register_object != "routeApp"));

    let route_edges = collect_route_edges(
        &root,
        &tsconfig,
        &all_files,
        Some(&facts),
        config_options.as_ref(),
    );
    assert!(route_edges.iter().any(|(from, to, kind)| {
        *kind == EdgeKind::RouteRef
            && from.as_file() == Some(client.as_path())
            && to.as_file() == Some(route_def.as_path())
    }));
    assert!(route_edges.iter().all(|(_from, to, kind)| {
        *kind != EdgeKind::RouteRef || to.as_file() != Some(http_def.as_path())
    }));

    let http_edges = collect_http_call_edges(
        &root,
        &tsconfig,
        Some(&facts),
        &[],
        &all_files,
        &all_files,
        config_options.as_ref(),
    );
    assert!(http_edges.iter().any(|(from, to, kind)| {
        *kind == EdgeKind::HttpCall
            && from.as_file() == Some(client.as_path())
            && to.as_file() == Some(http_def.as_path())
    }));
    assert!(http_edges.iter().all(|(_from, to, kind)| {
        *kind != EdgeKind::HttpCall || to.as_file() != Some(route_def.as_path())
    }));
}

#[test]
fn graph_config_helpers_require_explicit_prefixes_and_valid_globs() {
    let empty = crate::codebase::ts_resolver::normalize_path(&fixture("graph-empty-route-config"));
    let empty_options = graph_config_options(&empty).unwrap();
    assert!(resolved_backend_prefixes(&empty_options).is_empty());
    assert!(route_backend_prefixes(&empty_options).is_empty());

    let plan = GraphBuildPlan {
        routes: true,
        queues: true,
        http: true,
        ..GraphBuildPlan::default()
    };
    let context = ts_fact_context_from_options(&empty, plan, Some(&empty_options));
    assert!(context.backend_route_extractors.is_empty());
    assert!(context.queue_factory_glob.is_none());
    assert!(context.http_prefixes.is_empty());
    let context_without_options = ts_fact_context_from_options(&empty, plan, None);
    assert!(context_without_options.backend_route_extractors.is_empty());

    let mut manual_context = TsFactContext::new(&empty);
    add_backend_route_extractor(
        &mut manual_context,
        None,
        Some("backend/**/*.mts".to_string()),
    );
    add_backend_route_extractor(&mut manual_context, Some("app".to_string()), None);
    add_backend_route_extractor(
        &mut manual_context,
        Some("app".to_string()),
        Some("[".to_string()),
    );
    assert!(manual_context.backend_route_extractors.is_empty());

    assert!(compile_graph_glob("").is_none());
    assert!(compile_graph_glob("[").is_none());
    assert!(compile_graph_glob("backend/**/*.mts")
        .expect("valid graph glob should compile")
        .is_match(Path::new("backend/api/users.mts")));

    let explicit =
        crate::codebase::ts_resolver::normalize_path(&fixture("graph-default-route-config"));
    let explicit_options = graph_config_options(&explicit).unwrap();
    assert_eq!(
        resolved_backend_prefixes(&explicit_options),
        vec!["/api/".to_string()]
    );
    assert_eq!(
        route_backend_prefixes(&explicit_options),
        vec!["/api/".to_string()]
    );

    let missing_register_options = GraphConfigOptions {
        route: crate::codebase::config::RouteOptions::default(),
        queue: crate::codebase::config::QueueOptions::default(),
        http_route: crate::codebase::config::HttpRouteOptions {
            backend_pattern: "backend/**/*.mts".to_string(),
            register_object: String::new(),
        },
        http_call: crate::codebase::config::HttpCallOptions {
            backend_prefixes: vec!["/api/".to_string()],
        },
    };
    let invalid_glob_options = GraphConfigOptions {
        route: crate::codebase::config::RouteOptions::default(),
        queue: crate::codebase::config::QueueOptions::default(),
        http_route: crate::codebase::config::HttpRouteOptions {
            backend_pattern: "[".to_string(),
            register_object: "app".to_string(),
        },
        http_call: crate::codebase::config::HttpCallOptions {
            backend_prefixes: vec!["/api/".to_string()],
        },
    };
    let tsconfig =
        crate::codebase::ts_resolver::load_tsconfig(&explicit.join("tsconfig.json")).unwrap();
    let resolver = crate::codebase::ts_resolver::ImportResolver::new(&tsconfig);
    assert!(
        collect_route_edges(&explicit, &tsconfig, &[], None, Some(&explicit_options),).is_empty()
    );
    assert!(collect_http_call_edges(
        &explicit,
        &tsconfig,
        None,
        &[],
        &[],
        &[],
        Some(&explicit_options),
    )
    .is_empty());

    let queue_options = GraphConfigOptions {
        route: crate::codebase::config::RouteOptions::default(),
        queue: crate::codebase::config::QueueOptions {
            queue_pattern: "src/**/*.ts".to_string(),
            factory_specifier: "@app/queue".to_string(),
            factory_function: "createQueue".to_string(),
        },
        http_route: crate::codebase::config::HttpRouteOptions::default(),
        http_call: crate::codebase::config::HttpCallOptions::default(),
    };
    let mut forward = EdgeMap::new();
    let mut reverse = EdgeMap::new();
    add_queue_edges(
        &explicit,
        &resolver,
        &[],
        None,
        Some(&queue_options),
        &mut forward,
        &mut reverse,
    );
    assert!(forward.is_empty());

    assert!(collect_http_call_edges(
        &explicit,
        &tsconfig,
        None,
        &[],
        &[],
        &[],
        Some(&missing_register_options),
    )
    .is_empty());
    assert!(collect_http_call_edges(
        &explicit,
        &tsconfig,
        None,
        &[],
        &[],
        &[],
        Some(&invalid_glob_options),
    )
    .is_empty());
}

#[test]
fn effective_fact_plan_skips_config_dependent_domains_without_required_config() {
    let requested = GraphBuildPlan {
        routes: true,
        queues: true,
        http: true,
        ..GraphBuildPlan::default()
    };
    assert!(effective_ts_fact_plan(requested, None).is_empty());

    let empty = crate::codebase::ts_resolver::normalize_path(&fixture("graph-empty-route-config"));
    let empty_options = graph_config_options(&empty).unwrap();
    assert!(effective_ts_fact_plan(requested, Some(&empty_options)).is_empty());

    let explicit =
        crate::codebase::ts_resolver::normalize_path(&fixture("graph-default-route-config"));
    let explicit_options = graph_config_options(&explicit).unwrap();
    let route_and_http = effective_ts_fact_plan(requested, Some(&explicit_options));
    assert!(route_and_http.route_refs);
    assert!(route_and_http.backend_routes);
    assert!(route_and_http.http_calls);
    assert!(!route_and_http.symbols);
    assert!(!route_and_http.queue_usage);
    assert!(!route_and_http.queue_factory);

    let queue_options = GraphConfigOptions {
        route: crate::codebase::config::RouteOptions::default(),
        queue: crate::codebase::config::QueueOptions {
            queue_pattern: "src/**/*.ts".to_string(),
            factory_specifier: "@app/queue".to_string(),
            factory_function: "createQueue".to_string(),
        },
        http_route: crate::codebase::config::HttpRouteOptions::default(),
        http_call: crate::codebase::config::HttpCallOptions::default(),
    };
    let queue_only = effective_ts_fact_plan(
        GraphBuildPlan {
            queues: true,
            ..GraphBuildPlan::default()
        },
        Some(&queue_options),
    );
    assert!(queue_only.symbols);
    assert!(queue_only.queue_usage);
    assert!(queue_only.queue_factory);
}
