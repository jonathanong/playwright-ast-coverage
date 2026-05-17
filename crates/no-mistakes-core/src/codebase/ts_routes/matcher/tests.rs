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
fn param_does_not_match_empty_segment() {
    assert!(!matches("/", "/:id"));
}

#[test]
fn wildcard_match() {
    assert!(matches("/api/v1/anything", "/api/v1/*"));
}

#[test]
fn wildcard_requires_one_segment() {
    assert!(!matches("/api/v1", "/api/v1/*"));
}

#[test]
fn wildcard_does_not_match_empty_segment() {
    assert!(!matches("/", "/*"));
}

#[test]
fn wildcard_matches_multiple_segments() {
    assert!(matches("/api/v1/anything/nested", "/api/v1/*"));
}

#[test]
fn wildcard_matches_mid_pattern_segment() {
    assert!(matches("/api/v1/users", "/api/*/users"));
}

#[test]
fn wildcard_mid_pattern_matches_only_one_segment() {
    assert!(!matches("/api/v1/admin/users", "/api/*/users"));
}

#[test]
fn optional_wildcard_matches_zero_segments() {
    assert!(matches("/api/v1", "/api/v1/**"));
}

#[test]
fn optional_wildcard_matches_multiple_segments() {
    assert!(matches("/api/v1/anything/nested", "/api/v1/**"));
}

#[test]
fn length_mismatch() {
    assert!(!matches("/api/v1", "/api/v1/users"));
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

#[test]
fn matches_any_reports_any_matching_pattern() {
    let patterns = vec!["/nope".to_string(), "/api/:id".to_string()];

    assert!(matches_any("/api/42", &patterns));
    assert!(!matches_any("/other", &patterns));
}

#[test]
fn double_star_matches_empty_reference_tail_only_when_final() {
    assert!(matches("/api", "/api/**"));
    assert!(!matches("/api", "/api/**/users"));
    assert!(matches("/api/v1/admin/users", "/api/**/users"));
}
