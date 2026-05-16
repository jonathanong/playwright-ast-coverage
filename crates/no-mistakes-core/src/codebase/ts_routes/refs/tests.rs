use super::*;

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
