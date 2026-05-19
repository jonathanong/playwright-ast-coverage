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
}
