use super::*;
use crate::react_traits::report::types::{ComponentRef, Environment, FetchCall};
use std::collections::HashMap;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/react-traits-components")
        .join(name)
}

#[test]
fn run_analyze_inner_covers_root_fallback_child_aggregation_and_errors() {
    let root = fixture("nested");
    let file_config = FileConfig {
        frontend_root: Some("app".to_string()),
        assert_no_fetch: None,
    };

    let from_frontend = run_analyze_inner(
        &root,
        &file_config,
        &["components/Parent.tsx".to_string()],
        None,
    )
    .unwrap();
    let parent = from_frontend
        .iter()
        .find(|component| component.name == "default")
        .unwrap();
    assert!(parent
        .inherited_from_children
        .as_ref()
        .is_some_and(|facts| facts.has_fetch));

    let from_root = run_analyze_inner(
        &root,
        &file_config,
        &["app/components/Parent.tsx".to_string()],
        None,
    )
    .unwrap();
    assert!(from_root
        .iter()
        .any(|component| component.name == "default"));

    let missing = run_analyze_inner(
        &root.join("missing"),
        &file_config,
        &["components/Parent.tsx".to_string()],
        None,
    );
    assert!(missing.is_err());

    let invalid_glob = run_analyze_inner(&root, &file_config, &["[".to_string()], None);
    assert!(invalid_glob.is_err());
}

#[test]
fn run_analyze_inner_rejects_invalid_target_globs() {
    let root = fixture("nested");
    let file_config = FileConfig {
        frontend_root: Some("app".to_string()),
        assert_no_fetch: None,
    };

    let err = run_analyze_inner(&root, &file_config, &["[".to_string()], None).unwrap_err();

    assert!(format!("{err:#}").contains("["));
}

fn component(name: &str, file: &str) -> ComponentFacts {
    ComponentFacts {
        name: name.to_string(),
        file: file.to_string(),
        environment: Environment::Shared,
        has_state: false,
        has_props: false,
        passes_props: false,
        uses_memo: false,
        uses_context_provider: false,
        uses_suspense: false,
        fetches: Vec::new(),
        dependencies: Vec::new(),
        children: Vec::new(),
        inherited_from_children: None,
    }
}

#[test]
fn aggregate_children_skips_repeated_refs_and_unreadable_children() {
    let root = fixture("nested");
    let mut parent = component("Parent", "app/components/Parent.tsx");
    parent.children = vec![
        ComponentRef {
            file: "app/components/Child.tsx".to_string(),
            name: "Child".to_string(),
        },
        ComponentRef {
            file: "app/components/Child.tsx".to_string(),
            name: "Child".to_string(),
        },
        ComponentRef {
            file: "app/components/Missing.tsx".to_string(),
            name: "Missing".to_string(),
        },
        ComponentRef {
            file: "app/components/Child.tsx".to_string(),
            name: "NotChild".to_string(),
        },
    ];

    let mut child = component("Child", "app/components/Child.tsx");
    child.fetches.push(FetchCall {
        file: child.file.clone(),
        exported_name: None,
        shape: None,
    });
    let mut cache = HashMap::from([(root.join("app/components/Child.tsx"), vec![child])]);
    let agg = aggregate_children(&parent, &mut cache, &root, &mut HashSet::new());

    assert!(agg.has_fetch);
}
