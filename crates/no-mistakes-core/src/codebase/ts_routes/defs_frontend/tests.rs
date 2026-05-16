use super::*;

#[test]
fn root_page_maps_to_slash() {
    let p = Path::new("page.tsx");
    assert_eq!(path_to_route_pattern(p), "/");
}

#[test]
fn route_group_is_skipped() {
    let p = Path::new("(user)/user/[idOrUsername]/page.tsx");
    assert_eq!(path_to_route_pattern(p), "/user/:idOrUsername");
}

#[test]
fn slug_dynamic_segment() {
    let p = Path::new("communities/[slug]/settings/page.tsx");
    assert_eq!(path_to_route_pattern(p), "/communities/:slug/settings");
}

#[test]
fn catch_all_maps_to_wildcard() {
    let p = Path::new("[...rest]/page.tsx");
    assert_eq!(path_to_route_pattern(p), "/*");
}

#[test]
fn optional_catch_all_maps_to_optional_wildcard() {
    let p = Path::new("docs/[[...rest]]/page.tsx");
    assert_eq!(path_to_route_pattern(p), "/docs/**");
}

#[test]
fn parallel_and_private_segments_are_skipped() {
    let p = Path::new("@modal/_drafts/[id]/page.tsx");
    assert_eq!(path_to_route_pattern(p), "/:id");
}

#[test]
fn intercepting_prefix_is_stripped() {
    let p = Path::new("(.)photo/[id]/page.tsx");
    assert_eq!(path_to_route_pattern(p), "/photo/:id");
}

#[test]
fn intercepting_prefix_then_private_or_parallel_segment_is_skipped() {
    let p = Path::new("(.)_drafts/(..)@modal/[id]/page.tsx");
    assert_eq!(path_to_route_pattern(p), "/:id");
}

#[test]
fn two_level_intercepting_prefix_is_stripped() {
    let p = Path::new("(..)(..)photos/[id]/page.tsx");
    assert_eq!(path_to_route_pattern(p), "/photos/:id");
}

#[test]
fn static_nested_path() {
    let p = Path::new("communities/page.tsx");
    assert_eq!(path_to_route_pattern(p), "/communities");
}

#[test]
fn collect_frontend_routes_finds_pages() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis")
        .join("routes")
        .join("good")
        .join("web")
        .join("app");
    let routes = collect_frontend_routes(&root);
    assert!(routes.contains(&"/communities".to_string()));
    assert!(routes.contains(&"/communities/:slug".to_string()));
    assert!(routes.contains(&"/user/:id".to_string()));
}
