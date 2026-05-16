use super::{load_root_and_config, run_analyze};
use crate::cli::{Cli, Command};
use std::path::PathBuf;

fn fixture(category: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(category)
        .join(name)
}

fn make_cli(root: PathBuf) -> Cli {
    Cli {
        root,
        config: None,
        json: false,
        command: Command::Analyze {
            targets: vec![],
            return_depth: None,
        },
    }
}

#[test]
fn load_root_and_config_basic() {
    let fixture_root = fixture("react-traits-components", "basic");
    let cli = make_cli(PathBuf::from("."));
    let (root, config) = load_root_and_config(&fixture_root, &cli).expect("should load config");
    assert_eq!(root, fixture_root.join("."));
    assert_eq!(config.frontend_root.as_deref(), Some("app"));
}

#[test]
fn load_root_and_config_with_react_traits_key() {
    let fixture_root = fixture("react-traits-config", "assert-no-fetch");
    let cli = make_cli(PathBuf::from("."));
    let (_root, config) = load_root_and_config(&fixture_root, &cli).expect("should load config");
    assert_eq!(config.assert_no_fetch, Some(true));
    assert_eq!(config.frontend_root.as_deref(), Some("app"));
}

#[test]
fn run_analyze_basic_greeting() {
    let fixture_root = fixture("react-traits-components", "basic");
    let cli = make_cli(PathBuf::from("."));
    let targets = vec!["app/components/Greeting.tsx".to_string()];
    let results = run_analyze(&fixture_root, &cli, &targets, None).expect("should analyze");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "default");
    assert!(!results[0].has_state);
}

#[test]
fn run_analyze_nested_aggregates_child_fetch() {
    let fixture_root = fixture("react-traits-components", "nested");
    let cli = make_cli(PathBuf::from("."));
    let targets = vec![
        "app/components/Parent.tsx".to_string(),
        "app/components/Child.tsx".to_string(),
    ];
    let results = run_analyze(&fixture_root, &cli, &targets, None).expect("should analyze");
    let parent = results
        .iter()
        .find(|f| f.file.contains("Parent"))
        .expect("Parent not found");
    assert!(
        parent
            .inherited_from_children
            .as_ref()
            .is_some_and(|agg| agg.has_fetch),
        "Parent should inherit has_fetch from Child"
    );
}

#[test]
fn run_analyze_no_targets_returns_empty() {
    // When no target patterns are given, expand_globs with an empty pattern list
    // returns nothing, so run_analyze returns an empty vec.
    let fixture_root = fixture("react-traits-components", "basic");
    let cli = make_cli(PathBuf::from("."));
    let results = run_analyze(&fixture_root, &cli, &[], None).expect("should analyze");
    assert!(results.is_empty(), "empty patterns should yield no results");
}

#[test]
fn run_analyze_repeated_child_exercises_cycle_detection() {
    // Parent renders Child twice, so Child appears twice in the children list.
    // The second occurrence hits the visited.contains(&key) branch (line 64).
    let fixture_root = fixture("react-traits-components", "repeated-child");
    let cli = make_cli(PathBuf::from("."));
    let targets = vec![
        "app/components/Parent.tsx".to_string(),
        "app/components/Child.tsx".to_string(),
    ];
    let results = run_analyze(&fixture_root, &cli, &targets, None).expect("should analyze");
    let parent = results
        .iter()
        .find(|f| f.file.contains("Parent"))
        .expect("Parent not found");
    // Child has 2 refs but no fetch/state, so inherited_from_children stays None/default.
    assert!(
        parent.children.len() >= 2,
        "Parent should have at least 2 child refs"
    );
}

#[test]
fn run_analyze_target_not_at_root_falls_back_to_frontend_root() {
    // "components/Greeting.tsx" is not found at the root level,
    // so the code falls back to expand_globs(&frontend_root, targets).
    let fixture_root = fixture("react-traits-components", "basic");
    let cli = make_cli(PathBuf::from("."));
    let targets = vec!["components/Greeting.tsx".to_string()];
    let results = run_analyze(&fixture_root, &cli, &targets, None).expect("should analyze");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "default");
}

#[test]
fn run_analyze_child_not_in_cache_uses_canonicalize() {
    // When file_cache lookup by root.join(child_ref.file) fails (key mismatch),
    // the or_else canonicalize path is tried. Create a setup where the path lookup
    // needs canonicalization.
    let fixture_root = fixture("react-traits-components", "nested");
    let cli = make_cli(PathBuf::from("."));
    // Only analyze Parent, not Child — Child won't be in file_cache,
    // so aggregate_children will try canonicalize fallback (lines 69-73).
    let targets = vec!["app/components/Parent.tsx".to_string()];
    let results = run_analyze(&fixture_root, &cli, &targets, None).expect("should analyze");
    assert_eq!(results.len(), 1);
    // Child is not in file_cache so inherited facts won't include child's fetch.
}
