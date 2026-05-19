use super::*;
use std::path::PathBuf;

const API_PREFIXES: &[&str] = &["/api/", "/infra/", "/sitemaps/"];

fn fixture_source(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/ast-snippets/ts-http-calls")
        .join(name);
    std::fs::read_to_string(path).expect("HTTP call fixture source must be readable")
}

#[test]
fn detects_method_call_with_literal_path() {
    let src = r#"serverApi.get('/api/v1/topics', opts)"#;
    let calls = extract_http_calls(src, API_PREFIXES);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].path, "/api/v1/topics");
}

#[test]
fn detects_fetch_with_literal_path() {
    let src = r#"fetch('/api/v1/users', { method: 'GET' })"#;
    let calls = extract_http_calls(src, API_PREFIXES);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].path, "/api/v1/users");
}

#[test]
fn detects_nested_in_function() {
    let src = r#"
export function getUser(id: string) {
  return serverApi.get('/api/v1/users', { headers: {} })
}
"#;
    let calls = extract_http_calls(src, API_PREFIXES);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].path, "/api/v1/users");
}

#[test]
fn detects_inside_return() {
    let src = r#"
export function getTopic() {
  return wrapError(serverApi.get('/api/v1/topics'))
}
"#;
    let calls = extract_http_calls(src, API_PREFIXES);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].path, "/api/v1/topics");
}

#[test]
fn ignores_interpolated_template_literal_paths() {
    let src = r#"serverApi.get(`/api/v1/topics/${id}`)"#;
    let calls = extract_http_calls(src, API_PREFIXES);
    assert!(calls.is_empty());
}

#[test]
fn detects_static_template_literal_paths() {
    let src = r#"serverApi.get(`/api/v1/topics`)"#;
    let calls = extract_http_calls(src, API_PREFIXES);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].path, "/api/v1/topics");
}

#[test]
fn ignores_external_urls() {
    let src = r#"fetch('https://example.com/api/data')"#;
    let calls = extract_http_calls(src, API_PREFIXES);
    assert!(
        calls.is_empty(),
        "external URLs must not match /api/ prefix"
    );
}

#[test]
fn ignores_paths_not_matching_prefixes() {
    let src = r#"router.get('/v1/topics', handler)"#;
    let calls = extract_http_calls(src, API_PREFIXES);
    assert!(calls.is_empty(), "/v1/ is not in API_PREFIXES");
}

#[test]
fn detects_multiple_calls() {
    let src = r#"
const a = serverApi.get('/api/v1/users')
const b = serverApi.post('/api/v1/posts', data)
const c = serverApi.delete('/api/v1/comments')
"#;
    let calls = extract_http_calls(src, API_PREFIXES);
    assert_eq!(calls.len(), 3);
}

#[test]
fn detects_infra_prefix() {
    let src = r#"serverApi.get('/infra/ping')"#;
    let calls = extract_http_calls(src, API_PREFIXES);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].path, "/infra/ping");
}

#[test]
fn detects_inside_await_expression() {
    let src = r#"
async function load() {
  const data = await serverApi.get('/api/v1/topics')
  return data
}
"#;
    let calls = extract_http_calls(src, API_PREFIXES);
    assert_eq!(calls.len(), 1);
}

#[test]
fn detects_all_http_verbs() {
    let methods = [
        ("get", "/api/v1/res"),
        ("post", "/api/v1/res"),
        ("put", "/api/v1/res"),
        ("patch", "/api/v1/res"),
        ("delete", "/api/v1/res"),
        ("head", "/api/v1/res"),
        ("options", "/api/v1/res"),
    ];
    for (verb, path) in &methods {
        let src = format!("client.{verb}('{path}')");
        let calls = extract_http_calls(&src, API_PREFIXES);
        assert_eq!(calls.len(), 1, "failed for verb {verb}");
        assert_eq!(calls[0].path, *path);
    }
}

#[test]
fn fixture_walks_statement_and_expression_shapes() {
    let source = fixture_source("walk-all.tsx");
    let calls = extract_http_calls(&source, API_PREFIXES);
    let paths: Vec<_> = calls.iter().map(|call| call.path.as_str()).collect();

    for expected in [
        "/api/top",
        "/api/var",
        "/api/exported-function",
        "/api/default-arrow",
        "/api/default-function",
        "/api/if-test",
        "/api/if",
        "/api/else",
        "/api/try",
        "/api/catch",
        "/api/finally",
        "/api/for-init",
        "/api/for-test",
        "/api/for-update",
        "/api/for-body",
        "/api/for-in-right",
        "/api/for-in",
        "/api/for-of-right",
        "/api/for-of",
        "/api/while-test",
        "/api/while",
        "/api/do-while",
        "/api/do-while-test",
        "/api/arrow",
        "/api/conditional-test",
        "/api/conditional",
        "/api/alternate",
        "/api/logical",
        "/api/sequence-one",
        "/api/sequence-two",
        "/api/chained",
        "/api/casted",
        "/api/non-null",
    ] {
        assert!(paths.contains(&expected), "missing HTTP call {expected}");
    }
    assert!(!paths.contains(&"/other/top"));
}

#[test]
fn covers_sparse_statement_declaration_and_default_export_shapes() {
    let source = r#"
declare function ambient(): void;
export declare function exportedAmbient(): void;
export class Ignored {}
export default class IgnoredDefault {}
export default function defaultAmbient(): void;
export default client.get("/api/default-expression");

if (ready) {
  client.get("/api/if-only");
}

try {
  client.get("/api/try-only");
} finally {
  client.get("/api/finally");
}

for (i = client.get("/api/for-expr-init"); client.get("/api/for-test"); client.get("/api/for-update")) {
  client.get("/api/for-expr-body");
}
"#;
    let calls = extract_http_calls(source, API_PREFIXES);
    let paths: Vec<_> = calls.iter().map(|call| call.path.as_str()).collect();

    for expected in [
        "/api/if-only",
        "/api/try-only",
        "/api/finally",
        "/api/for-expr-init",
        "/api/for-test",
        "/api/for-update",
        "/api/for-expr-body",
    ] {
        assert!(paths.contains(&expected), "missing {expected}");
    }
    assert!(!paths.contains(&"/api/default-expression"));
}
