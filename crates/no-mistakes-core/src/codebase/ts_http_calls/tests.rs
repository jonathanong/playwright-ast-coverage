use super::*;

const API_PREFIXES: &[&str] = &["/api/", "/infra/", "/sitemaps/"];

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
