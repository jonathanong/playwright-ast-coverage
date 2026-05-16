use super::*;

#[test]
fn normalizes_params_and_joins_prefixes() {
    assert_eq!(normalize_route("/api/v1/users/:id"), "/api/v1/users/*");
    assert_eq!(normalize_route("/files/{/*path}"), "/files/**");
    assert_eq!(join_paths("/api", "/users"), "/api/users");
    assert_eq!(join_paths("/api/", "users/"), "/api/users");
}
