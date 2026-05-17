use super::*;
use std::path::PathBuf;

fn route_fixture_source(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/ast-snippets/ts-routes")
        .join(name);
    std::fs::read_to_string(path).expect("route fixture source must be readable")
}

#[test]
fn extracts_simple_href() {
    let source = r#"const x = <a href="/communities">Communities</a>;"#;
    let refs = extract_route_refs(source, "nav.tsx");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].pattern, "/communities");
}

#[test]
fn extracts_router_push() {
    let source = "const router = useRouter();\nrouter.push('/source');";
    let refs = extract_route_refs(source, "comp.tsx");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].pattern, "/source");
}

#[test]
fn ignores_unbound_router_push() {
    let source = "router.push('/source');";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn extracts_router_prefetch() {
    let source = "const router = useRouter();\nrouter.prefetch('/source');";
    let refs = extract_route_refs(source, "comp.tsx");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].pattern, "/source");
}

#[test]
fn extracts_use_router_binding() {
    // Any variable bound to useRouter() should be recognized as a router.
    let source = "const nav = useRouter();\nnav.push('/dashboard');";
    let refs = extract_route_refs(source, "comp.tsx");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].pattern, "/dashboard");
}

#[test]
fn extracts_router_binding_declared_after_handler() {
    let source = "function Component() {\nconst onClick = () => router.push('/dashboard');\nconst router = useRouter();\n}";
    let refs = extract_route_refs(source, "comp.tsx");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].pattern, "/dashboard");
}

#[test]
fn extracts_destructured_use_router_method() {
    let source = "const { push } = useRouter();\npush('/dashboard');";
    let refs = extract_route_refs(source, "comp.tsx");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].pattern, "/dashboard");
}

#[test]
fn extracts_function_scoped_destructured_use_router_method() {
    let source = "function Component() {\nconst { push } = useRouter();\npush('/dashboard');\n}";
    let refs = extract_route_refs(source, "comp.tsx");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].pattern, "/dashboard");
}

#[test]
fn extracts_aliased_destructured_use_router_method() {
    let source = "const { push: navigate } = useRouter();\nnavigate('/dashboard');";
    let refs = extract_route_refs(source, "comp.tsx");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].pattern, "/dashboard");
}

#[test]
fn extracts_defaulted_destructured_use_router_method() {
    let source = "const { push = fallback } = useRouter();\npush('/dashboard');";
    let refs = extract_route_refs(source, "comp.tsx");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].pattern, "/dashboard");
}

#[test]
fn extracts_defaulted_aliased_destructured_use_router_method() {
    let source = "const { push: navigate = fallback } = useRouter();\nnavigate('/dashboard');";
    let refs = extract_route_refs(source, "comp.tsx");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].pattern, "/dashboard");
}

#[test]
fn ignores_alias_from_untracked_router_method() {
    let source = "const { back: push } = useRouter();\npush('/dashboard');";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn extracts_redirect_call() {
    let source = "import { redirect } from 'next/navigation';\nredirect('/login');";
    let refs = extract_route_refs(source, "comp.tsx");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].pattern, "/login");
}

#[test]
fn ignores_unimported_redirect_call() {
    let source = "redirect('/login');";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn ignores_shadowed_redirect_parameter() {
    let source =
        "import { redirect } from 'next/navigation';\nfunction helper(redirect) { redirect('/login'); }";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn ignores_redirect_shadowed_by_use_router_binding() {
    let source =
        "import { redirect } from 'next/navigation';\nfunction Component() {\nredirect('/login');\nconst redirect = useRouter();\n}";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn ignores_redirect_shadowed_by_uninitialized_binding() {
    let source =
        "import { redirect } from 'next/navigation';\nfunction Component() {\nlet redirect;\nredirect('/login');\n}";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn ignores_redirect_shadowed_by_function_declaration() {
    let source =
        "import { redirect } from 'next/navigation';\nfunction Component() {\nfunction redirect() {}\nredirect('/login');\n}";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn ignores_redirect_shadowed_before_hoisted_var_declaration() {
    let source =
        "import { redirect } from 'next/navigation';\nfunction Component() {\nconst onClick = () => redirect('/login');\nvar redirect = localFn;\nonClick();\n}";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn ignores_redirect_shadowed_by_var_inside_block_after_block() {
    let source =
        "import { redirect } from 'next/navigation';\nfunction Component() {\nif (ready) { var redirect = localFn; }\nredirect('/login');\n}";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn ignores_redirect_shadowed_by_var_inside_for_after_loop() {
    let source =
        "import { redirect } from 'next/navigation';\nfunction Component() {\nfor (var redirect = localFn; ready; ready = false) {}\nredirect('/login');\n}";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn ignores_redirect_shadowed_by_var_inside_for_in_after_loop() {
    let source =
        "import { redirect } from 'next/navigation';\nfunction Component() {\nfor (var redirect in routes) {}\nredirect('/login');\n}";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn ignores_router_method_shadowed_by_var_inside_for_of_after_loop() {
    let source =
        "function Component() {\nconst { push } = useRouter();\nfor (var push of handlers) {}\npush('/dashboard');\n}";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn keeps_redirect_binding_after_let_for_of_shadow() {
    let source =
        "import { redirect } from 'next/navigation';\nfor (let redirect of items) {}\nredirect('/dashboard');";
    let refs = extract_route_refs(source, "comp.tsx");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].pattern, "/dashboard");
}

#[test]
fn does_not_leak_const_router_binding_after_for_loop() {
    let source =
        "function Component() {\nfor (const router = useRouter(); ready; ready = false) {}\nrouter.push('/dashboard');\n}";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn ignores_redirect_shadowed_by_class_declaration() {
    let source =
        "import { redirect } from 'next/navigation';\nfunction Component() {\nclass redirect {}\nredirect('/login');\n}";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn ignores_router_method_shadowed_by_class_declaration() {
    let source =
        "function Component() {\nconst { push } = useRouter();\nclass push {}\npush('/dashboard');\n}";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn ignores_redirect_shadowed_by_named_function_expression() {
    let source =
        "import { redirect } from 'next/navigation';\nconst helper = function redirect() { redirect('/login'); };";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn ignores_shadowed_router_method_variable() {
    let source =
        "function Component() {\nconst { push } = useRouter();\nif (ready) { const push = run; push('/dashboard'); }\n}";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn ignores_router_method_shadowed_by_use_router_binding() {
    let source =
        "function Component() {\nconst { push } = useRouter();\nif (ready) { push('/dashboard'); const push = useRouter(); }\n}";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn ignores_router_method_shadowed_by_function_declaration() {
    let source =
        "function Component() {\nconst { push } = useRouter();\nfunction push() {}\npush('/dashboard');\n}";
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(refs.is_empty());
}

#[test]
fn extracts_template_href() {
    let source = r#"const x = <a href={`/communities/${slug}`}>Community</a>;"#;
    let refs = extract_route_refs(source, "nav.tsx");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].pattern, "/communities/:param");
}

#[test]
fn extracts_object_href_pathname() {
    let source = r#"const x = <Link href={{ pathname: `/users/${id}` }}>User</Link>;"#;
    let refs = extract_route_refs(source, "nav.tsx");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].pattern, "/users/:param");
}

#[test]
fn extracts_object_href_next_pathname() {
    let source = r#"const x = <Link href={{ pathname: "/users/[id]" }}>User</Link>;"#;
    let refs = extract_route_refs(source, "nav.tsx");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].pattern, "/users/:id");
}

#[test]
fn extracts_fetch_local_path() {
    let source = "fetch('/api/v1/my/api-keys');";
    let refs = extract_route_refs(source, "comp.tsx");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].pattern, "/api/v1/my/api-keys");
}

#[test]
fn fetch_external_url_skipped() {
    // fetch() to an external URL should NOT be captured.
    let source = r#"fetch('https://api.stripe.com/v1/charges');"#;
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(
        refs.is_empty(),
        "external fetch URL should not be a route ref"
    );
}

#[test]
fn fetch_relative_path_skipped() {
    // fetch() with a relative path (no leading slash) should not be captured.
    let source = r#"fetch('data.json');"#;
    let refs = extract_route_refs(source, "comp.tsx");
    assert!(
        refs.is_empty(),
        "relative-path fetch should not be a route ref"
    );
}

#[test]
fn skips_external_href() {
    let source = r#"const x = <a href="https://example.com/foo">Ext</a>;"#;
    let refs = extract_route_refs(source, "nav.tsx");
    assert!(refs.is_empty());
}

#[test]
fn should_skip_checks() {
    assert!(should_skip("http://example.com"));
    assert!(should_skip("https://example.com"));
    assert!(should_skip("//example.com"));
    assert!(should_skip("?query=param"));
    assert!(should_skip("#anchor"));
    assert!(should_skip(""));
    assert!(should_skip(":param/rest"));
    assert!(!should_skip("/communities"));
}

#[test]
fn fixture_refs_walker_covers_router_jsx_fetch_and_scope_shapes() {
    let source = route_fixture_source("refs-walk-all.tsx");
    let refs = extract_route_refs(&source, "fixture.tsx");
    let patterns: Vec<&str> = refs
        .iter()
        .map(|route_ref| route_ref.pattern.as_str())
        .collect();

    for expected in [
        "/router",
        "/router/:id",
        "/prefetch/:param",
        "/member-router",
        "/method",
        "/replace",
        "/redirect",
        "/api/local",
        "/api/member",
        "/spread",
        "/spread-attr",
        "/href",
        "/to/:param",
        "/string-key/:slug/",
        "/catch-all/*",
        "/optional/**",
        "/after-spread",
        "/namespaced",
        "/nested-fragment",
        "/if",
        "/else",
        "/conditional",
        "/alternate",
        "/sequence-one",
        "/sequence-two",
        "/assignment",
        "/assertion-call",
        "/satisfies",
        "/non-null",
        "/parenthesized",
        "/return",
        "/arrow",
        "/function-expression",
        "/export-var",
        "/export-var-init",
        "/export-function",
        "/default-expression",
        "/default-export",
        "/for-var-router",
        "/after-let-for-of",
        "/after-class-shadow-check",
        "/exported-function-body",
        "/default-function-body/:id",
        "/do-while-after",
    ] {
        assert!(
            patterns.contains(&expected),
            "expected {expected} in {patterns:?}"
        );
    }
    assert!(
        !patterns.iter().any(|pattern| pattern.contains("ignored")),
        "unexpected ignored route in {patterns:?}"
    );
    assert!(!patterns.contains(&"?query"));
}

#[test]
fn extracts_from_default_exports_and_jsx_edge_shapes() {
    let cases = [
        (
            r#"
            const router = useRouter();
            export default function DefaultFn(...rest) {
              return <><Link href={}></Link>{<a href="/fragment-child">x</a>}</>;
            }
            "#,
            "/fragment-child",
        ),
        (
            r#"
            const router = useRouter();
            export default () => router.push("/default-arrow-expression");
            "#,
            "/default-arrow-expression",
        ),
        (
            r#"
            const router = useRouter();
            const view = <ns:Link href="https://example.com/ignored-ns" />;
            router.push(dynamic);
            api.fetch(dynamic);
            "#,
            "/none",
        ),
    ];

    let refs = extract_route_refs(cases[0].0, "default-fn.tsx");
    assert!(refs.iter().any(|route_ref| route_ref.pattern == cases[0].1));

    let refs = extract_route_refs(cases[1].0, "default-arrow.tsx");
    assert!(refs.iter().any(|route_ref| route_ref.pattern == cases[1].1));

    let refs = extract_route_refs(cases[2].0, "ignored.tsx");
    assert!(refs.is_empty());
}
