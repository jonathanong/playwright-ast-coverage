use super::*;

#[test]
fn empty_patterns_return_no_files() {
    let files = expand_globs(Path::new("."), &[]).expect("empty globs should succeed");
    assert!(files.is_empty());
}

#[test]
fn skip_dir_matches_generated_and_dependency_directories() {
    for name in [
        ".git",
        "node_modules",
        "target",
        "dist",
        "build",
        "coverage",
    ] {
        assert!(is_skip_dir(Path::new(name)), "{name}");
    }
    assert!(!is_skip_dir(Path::new("app")));
}
