use super::{resolve_optional_root, resolve_root};
use std::path::Path;

#[test]
fn resolve_root_preserves_absolute_paths() {
    let cwd = Path::new("/repo");
    let root = Path::new("/workspace/app");

    assert_eq!(resolve_root(root, cwd), root);
}

#[test]
fn resolve_root_joins_relative_paths() {
    assert_eq!(
        resolve_root(Path::new("app"), Path::new("/repo")),
        Path::new("/repo/app")
    );
}

#[test]
fn resolve_optional_root_defaults_to_cwd() {
    let cwd = Path::new("/repo");

    assert_eq!(resolve_optional_root(None, cwd), cwd);
}

#[test]
fn resolve_optional_root_resolves_provided_root() {
    assert_eq!(
        resolve_optional_root(Some(Path::new("app")), Path::new("/repo")),
        Path::new("/repo/app")
    );
}
