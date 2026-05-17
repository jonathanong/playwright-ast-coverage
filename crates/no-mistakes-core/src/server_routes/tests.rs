use super::*;
use crate::server_routes::model::{Binding, FileFacts, ImportBinding, MountSite, RouteSite};
use std::collections::HashMap;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/server-ast-routes")
        .join(name)
}

#[test]
fn express_project_reports_route_edges() {
    let report = analyze_project(&fixture("express"), None, &[]).unwrap();
    assert!(report
        .routes
        .iter()
        .any(|route| route.route == "/api/v1/users/*" && route.method == "get"));
    assert!(report
        .edges
        .iter()
        .any(|edge| edge.from == "backend/api/users.ts" && edge.to == "/api/v1/users/*"));
}

#[test]
fn hono_project_reports_prefixed_routes() {
    let report = analyze_project(&fixture("hono"), None, &[]).unwrap();
    assert!(report
        .routes
        .iter()
        .any(|route| route.route == "/api/posts/*" && route.method == "get"));
    assert!(report
        .routes
        .iter()
        .any(|route| { route.route == "/api/posts/*/comments" && route.method == "get" }));
    assert!(report
        .routes
        .iter()
        .any(|route| route.route == "/api/posts/*/likes" && route.method == "post"));
}

#[test]
fn koa_router_named_routes_and_mounts_are_supported() {
    let report = analyze_project(&fixture("koa-router"), None, &[]).unwrap();
    assert!(report
        .routes
        .iter()
        .any(|route| route.route == "/api/users/*" && route.method == "delete"));
    assert!(report
        .routes
        .iter()
        .any(|route| route.route == "/api/users/*/profile" && route.method == "get"));
}

#[test]
fn related_crosses_route_edges() {
    let report = analyze_project(&fixture("express"), None, &[]).unwrap();
    let edges = related(
        &report,
        &["backend/api/users.ts".to_string()],
        RelatedDirection::Deps,
    );
    assert!(edges.iter().any(|edge| edge.to == "/api/v1/users/*"));
}

#[test]
fn filters_limit_discovered_sources() {
    let report = analyze_project(
        &fixture("express"),
        None,
        &["backend/api/users.ts".to_string()],
    )
    .unwrap();
    assert_eq!(report.summary.total_files, 1);
}

#[test]
fn mixed_framework_shapes_are_supported() {
    let report = analyze_project(&fixture("mixed"), None, &[]).unwrap();
    for expected in [
        "/array/*",
        "/array/*/edit",
        "/api-server/*",
        "/books/*",
        "/matched/*",
        "/v1/koa/*",
        "/child/hono-child/*",
        "/paren/*",
        "/v1/shared/status",
        "/v2/shared/status",
    ] {
        assert!(
            report.routes.iter().any(|route| route.route == expected),
            "missing {expected}"
        );
    }
    assert!(!report
        .routes
        .iter()
        .any(|route| route.route == "not-a-route"));
}

#[test]
fn modular_mounts_apply_prefixes_across_files() {
    let report = analyze_project(&fixture("modular"), None, &[]).unwrap();
    assert!(report
        .routes
        .iter()
        .any(|route| route.route == "/api/*" && route.file == "backend/api/users.ts"));
    assert!(report
        .routes
        .iter()
        .any(|route| route.route == "/admin" && route.file == "backend/api/admin.ts"));
}

#[test]
fn related_dependents_and_both_are_supported() {
    let report = analyze_project(&fixture("mixed"), None, &[]).unwrap();
    let dependents = related(
        &report,
        &["/api-server/*".to_string()],
        RelatedDirection::Dependents,
    );
    assert!(dependents
        .iter()
        .any(|edge| edge.to == "backend/api/routes.ts"));
    let both = related(
        &report,
        &["backend/api/routes.ts".to_string()],
        RelatedDirection::Both,
    );
    assert!(both.iter().any(|edge| edge.to == "/matched/*"));
}

#[test]
fn normalize_handles_root_queries_braced_wildcards_and_empty_joins() {
    assert_eq!(normalize::join_paths("", "users/"), "/users");
    assert_eq!(normalize::join_paths("/", "/users?x#y"), "/users");
    assert_eq!(normalize::join_paths("/api", ""), "/api");
    assert_eq!(
        normalize::normalize_route("/files/{*rest}/tail"),
        "/files/**/tail"
    );
    assert_eq!(normalize::normalize_route(""), "/");
}

#[test]
fn related_with_unknown_root_stops_on_empty_frontier() {
    let report = analyze_project(&fixture("express"), None, &[]).unwrap();
    let edges = related(
        &report,
        &["backend/api/missing.ts".to_string()],
        RelatedDirection::Deps,
    );

    assert!(edges.is_empty());
}

#[test]
fn mount_resolver_covers_local_imported_fallback_and_recursive_prefixes() {
    let parent = PathBuf::from("/repo/parent.ts");
    let child = PathBuf::from("/repo/child.ts");
    let grand = PathBuf::from("/repo/grand/index.ts");

    let mut parent_facts = FileFacts::default();
    parent_facts.bindings.insert(
        "api".to_string(),
        Binding {
            framework: Framework::Express,
            prefixes: vec!["/root".to_string()],
        },
    );
    parent_facts.imports.push(ImportBinding {
        local: "childRouter".to_string(),
        imported: "named".to_string(),
        source: "./child".to_string(),
    });
    parent_facts.mounts.push(MountSite {
        parent: "api".to_string(),
        child: "childRouter".to_string(),
        prefix: "/api".to_string(),
    });

    let mut child_facts = FileFacts::default();
    child_facts.bindings.insert(
        "localRouter".to_string(),
        Binding {
            framework: Framework::Express,
            prefixes: vec!["/v1".to_string()],
        },
    );
    child_facts
        .exports
        .insert("named".to_string(), "localRouter".to_string());
    child_facts.imports.push(ImportBinding {
        local: "grandRouter".to_string(),
        imported: "missing".to_string(),
        source: "./grand".to_string(),
    });
    child_facts.mounts.push(MountSite {
        parent: "localRouter".to_string(),
        child: "grandRouter".to_string(),
        prefix: "/nested".to_string(),
    });

    let mut grand_facts = FileFacts::default();
    grand_facts.bindings.insert(
        "onlyRouter".to_string(),
        Binding {
            framework: Framework::Express,
            prefixes: vec![],
        },
    );
    grand_facts
        .exports
        .insert("only".to_string(), "onlyRouter".to_string());
    grand_facts.routes.push(RouteSite {
        file: grand.clone(),
        line: 1,
        binding: "onlyRouter".to_string(),
        method: "get".to_string(),
        raw_path: "/leaf".to_string(),
        path: "/leaf".to_string(),
        framework: Framework::Express,
    });

    let facts = HashMap::from([
        (parent, parent_facts),
        (child.clone(), child_facts),
        (grand.clone(), grand_facts),
    ]);
    let mounts = super::mounts::resolve_mounts(&facts);
    assert!(mounts
        .iter()
        .any(|mount| mount.child_file == child && mount.child == "localRouter"));
    assert!(mounts
        .iter()
        .any(|mount| mount.child_file == grand && mount.child == "onlyRouter"));

    let prefixes = super::mounts::prefixes_for(&facts[&grand].routes[0], &facts, &mounts);
    assert!(prefixes.contains(&"/api/nested".to_string()));
    assert!(prefixes.contains(&"/root/api/nested".to_string()));
}

#[test]
fn mount_resolver_ignores_unresolvable_and_non_relative_imports() {
    let parent = PathBuf::from("/repo/parent.ts");
    let mut parent_facts = FileFacts::default();
    parent_facts.mounts.push(MountSite {
        parent: "api".to_string(),
        child: "externalRouter".to_string(),
        prefix: "/external".to_string(),
    });
    parent_facts.imports.push(ImportBinding {
        local: "externalRouter".to_string(),
        imported: "default".to_string(),
        source: "pkg".to_string(),
    });

    let facts = HashMap::from([(parent, parent_facts)]);
    assert!(super::mounts::resolve_mounts(&facts).is_empty());
}

#[test]
fn mount_resolver_covers_import_binding_fallbacks_and_cycles() {
    let parent = PathBuf::from("/repo/parent.ts");
    let local = PathBuf::from("/repo/local.ts");
    let ambiguous = PathBuf::from("/repo/ambiguous.ts");

    let mut parent_facts = FileFacts::default();
    parent_facts.bindings.insert(
        "api".to_string(),
        Binding {
            framework: Framework::Express,
            prefixes: vec![],
        },
    );
    for child in [
        "sameName",
        "localExport",
        "localBinding",
        "aliasToExport",
        "aliasToBinding",
    ] {
        parent_facts.mounts.push(MountSite {
            parent: "api".to_string(),
            child: child.to_string(),
            prefix: format!("/{child}"),
        });
        parent_facts.imports.push(ImportBinding {
            local: child.to_string(),
            imported: if child.starts_with("alias") {
                "missing".to_string()
            } else {
                child.to_string()
            },
            source: "./local.ts".to_string(),
        });
    }
    parent_facts.mounts.push(MountSite {
        parent: "api".to_string(),
        child: "ambiguous".to_string(),
        prefix: "/ambiguous".to_string(),
    });
    parent_facts.imports.push(ImportBinding {
        local: "ambiguous".to_string(),
        imported: "missing".to_string(),
        source: "./ambiguous.ts".to_string(),
    });

    let mut local_facts = FileFacts::default();
    local_facts.bindings.insert(
        "sameName".to_string(),
        Binding {
            framework: Framework::Express,
            prefixes: vec![],
        },
    );
    local_facts.bindings.insert(
        "localBinding".to_string(),
        Binding {
            framework: Framework::Express,
            prefixes: vec![],
        },
    );
    local_facts.bindings.insert(
        "aliasToBinding".to_string(),
        Binding {
            framework: Framework::Express,
            prefixes: vec![],
        },
    );
    local_facts
        .exports
        .insert("localExport".to_string(), "localBinding".to_string());
    local_facts
        .exports
        .insert("aliasToExport".to_string(), "localBinding".to_string());

    let mut ambiguous_facts = FileFacts::default();
    ambiguous_facts
        .exports
        .insert("one".to_string(), "one".to_string());
    ambiguous_facts
        .exports
        .insert("two".to_string(), "two".to_string());

    let facts = HashMap::from([
        (parent, parent_facts),
        (local.clone(), local_facts),
        (ambiguous, ambiguous_facts),
    ]);
    let mounts = super::mounts::resolve_mounts(&facts);
    assert!(mounts
        .iter()
        .any(|mount| mount.child_file == local && mount.child == "sameName"));
    assert!(mounts
        .iter()
        .any(|mount| mount.child_file == local && mount.child == "localBinding"));

    let site = RouteSite {
        file: local.clone(),
        line: 1,
        binding: "localBinding".to_string(),
        method: "get".to_string(),
        raw_path: "/leaf".to_string(),
        path: "/leaf".to_string(),
        framework: Framework::Express,
    };
    let prefixes = super::mounts::prefixes_for(&site, &facts, &mounts);
    assert!(prefixes.iter().any(|prefix| prefix.contains("localExport")));

    let missing_parent = [super::mounts::ResolvedMount {
        parent_file: PathBuf::from("/repo/missing.ts"),
        parent: "missing".to_string(),
        child_file: local.clone(),
        child: "localBinding".to_string(),
        prefix: "/orphan".to_string(),
    }];
    let prefixes = super::mounts::prefixes_for(&site, &facts, &missing_parent);
    assert_eq!(prefixes, vec!["/orphan"]);
}

#[test]
fn report_builder_includes_diagnostics_and_dynamic_summary() {
    let root = PathBuf::from("/repo");
    let file = root.join("api.ts");
    let mut facts = FileFacts::default();
    facts.diagnostics.push((3, "unsupported route".to_string()));
    facts.bindings.insert(
        "api".to_string(),
        Binding {
            framework: Framework::Express,
            prefixes: vec![],
        },
    );
    facts.routes.push(RouteSite {
        file: file.clone(),
        line: 4,
        binding: "api".to_string(),
        method: "get".to_string(),
        raw_path: "/users/:id".to_string(),
        path: "/users/:id".to_string(),
        framework: Framework::Express,
    });

    let report = graph::build_report(&root, &HashMap::from([(file, facts)]));
    assert_eq!(report.diagnostics[0].file, "api.ts");
    assert_eq!(report.diagnostics[0].line, 3);
    assert_eq!(report.summary.dynamic_routes, 1);
}

#[test]
fn mount_resolver_covers_single_export_none_and_cycle_guards() {
    let parent = PathBuf::from("/repo/parent.ts");
    let child = PathBuf::from("/repo/child.ts");

    let mut parent_facts = FileFacts::default();
    parent_facts.bindings.insert(
        "api".to_string(),
        Binding {
            framework: Framework::Express,
            prefixes: vec!["/root".to_string()],
        },
    );
    for local in ["onlyDefault", "ambiguous"] {
        parent_facts.imports.push(ImportBinding {
            local: local.to_string(),
            imported: "default".to_string(),
            source: "./child.ts".to_string(),
        });
        parent_facts.mounts.push(MountSite {
            parent: "api".to_string(),
            child: local.to_string(),
            prefix: format!("/{local}"),
        });
    }

    let mut child_facts = FileFacts::default();
    child_facts.bindings.insert(
        "actual".to_string(),
        Binding {
            framework: Framework::Express,
            prefixes: vec![],
        },
    );
    child_facts
        .exports
        .insert("only".to_string(), "actual".to_string());

    let facts = HashMap::from([(parent.clone(), parent_facts), (child.clone(), child_facts)]);
    let mounts = super::mounts::resolve_mounts(&facts);
    assert!(mounts.iter().any(|mount| mount.child == "actual"));

    let cycle = [
        super::mounts::ResolvedMount {
            parent_file: parent.clone(),
            parent: "api".to_string(),
            child_file: child.clone(),
            child: "actual".to_string(),
            prefix: String::new(),
        },
        super::mounts::ResolvedMount {
            parent_file: child.clone(),
            parent: "actual".to_string(),
            child_file: parent.clone(),
            child: "api".to_string(),
            prefix: String::new(),
        },
    ];
    let site = RouteSite {
        file: child,
        line: 1,
        binding: "actual".to_string(),
        method: "get".to_string(),
        raw_path: "/leaf".to_string(),
        path: "/leaf".to_string(),
        framework: Framework::Express,
    };
    let prefixes = super::mounts::prefixes_for(&site, &facts, &cycle);
    assert!(prefixes.contains(&"/".to_string()));
    assert!(prefixes.contains(&"/root".to_string()));
}
