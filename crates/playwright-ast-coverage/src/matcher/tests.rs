use super::*;

#[test]
fn exact_match() {
    assert!(matches("/api/v1/users", "/api/v1/users"));
}

#[test]
fn param_match() {
    assert!(matches("/api/v1/users/42", "/api/v1/users/:id"));
}

#[test]
fn wildcard_match() {
    assert!(matches("/api/v1/anything", "/api/v1/*"));
}

#[test]
fn catch_all_and_optional_catch_all_match_remaining_segments() {
    assert!(matches("/docs/a/b", "/docs/*"));
    assert!(matches("/shop", "/shop/**"));
    assert!(matches("/shop/a/b", "/shop/**"));
}

#[test]
fn length_mismatch() {
    assert!(!matches("/api/v1", "/api/v1/users"));
}

#[test]
fn dynamic_segments_reject_empty_segments() {
    assert!(!matches("/users//settings", "/users/:id/settings"));
}

#[test]
fn literal_mismatch() {
    assert!(!matches("/api/v1/users", "/api/v1/posts"));
}

#[test]
fn query_stripped() {
    assert!(matches("/api/v1/users?foo=bar", "/api/v1/users"));
}

#[test]
fn fragment_stripped() {
    assert!(matches("/api/v1/users#section", "/api/v1/users"));
}

#[test]
fn trailing_slash_stripped() {
    assert!(matches("/api/v1/users/", "/api/v1/users"));
}

#[test]
fn root_slash_preserved() {
    assert!(matches("/", "/"));
}
