use super::expand_globs;
use std::path::PathBuf;

fn fixture(category: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(category)
        .join(name)
}

#[test]
fn expands_glob_finds_tsx_files() {
    let root = fixture("react-traits-components", "basic").join("app");
    let patterns = vec!["**/*.tsx".to_string()];
    let files = expand_globs(&root, &patterns).expect("should expand");
    assert!(!files.is_empty());
    assert!(files
        .iter()
        .all(|f| f.extension().and_then(|e| e.to_str()) == Some("tsx")));
}

#[test]
fn expands_glob_specific_file() {
    let root = fixture("react-traits-components", "basic").join("app");
    let patterns = vec!["components/Greeting.tsx".to_string()];
    let files = expand_globs(&root, &patterns).expect("should expand");
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("Greeting.tsx"));
}

#[test]
fn expands_empty_patterns_returns_empty() {
    let root = fixture("react-traits-components", "basic").join("app");
    let files = expand_globs(&root, &[]).expect("should expand");
    assert!(files.is_empty());
}

#[test]
fn skips_node_modules_directory() {
    let root = fixture("react-traits-glob", "skip-node-modules");
    let patterns = vec!["**/*.tsx".to_string()];
    let files = expand_globs(&root, &patterns).expect("should expand");
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("App.tsx"));
}

#[test]
fn invalid_glob_pattern_returns_error() {
    // A glob pattern with unmatched '[' is invalid and causes build() to return Err,
    // which exercises the `?` error branch at line 9.
    let root = fixture("react-traits-components", "basic").join("app");
    let patterns = vec!["[invalid".to_string()];
    let result = expand_globs(&root, &patterns);
    assert!(result.is_err(), "invalid glob should return error");
}

#[test]
fn skips_target_directory() {
    let root = fixture("react-traits-glob", "skip-target");
    let patterns = vec!["**/*.tsx".to_string()];
    let files = expand_globs(&root, &patterns).expect("should expand");
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("App.tsx"));
}
