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
    use std::fs;
    use tempfile::tempdir;
    let tmp = tempdir().unwrap();
    let nm = tmp.path().join("node_modules");
    fs::create_dir(&nm).unwrap();
    fs::write(nm.join("index.tsx"), "export default function Foo() {}").unwrap();
    fs::write(
        tmp.path().join("App.tsx"),
        "export default function App() {}",
    )
    .unwrap();
    let patterns = vec!["**/*.tsx".to_string()];
    let files = expand_globs(tmp.path(), &patterns).expect("should expand");
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("App.tsx"));
}

#[test]
fn skips_target_directory() {
    use std::fs;
    use tempfile::tempdir;
    let tmp = tempdir().unwrap();
    let target = tmp.path().join("target");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("index.tsx"), "export default function Foo() {}").unwrap();
    fs::write(
        tmp.path().join("App.tsx"),
        "export default function App() {}",
    )
    .unwrap();
    let patterns = vec!["**/*.tsx".to_string()];
    let files = expand_globs(tmp.path(), &patterns).expect("should expand");
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("App.tsx"));
}
