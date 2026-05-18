use super::*;
use crate::codebase::check_facts::{CheckFactMap, CheckFileFacts};
use crate::react_traits::analyze::file::FileAnalysis;
use crate::react_traits::report::types::{ComponentRef, Environment, FetchCall};
use std::collections::HashMap;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/react-traits-components")
        .join(name)
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

fn facts(entries: Vec<(PathBuf, Vec<ComponentFacts>)>) -> CheckFactMap {
    CheckFactMap {
        files: entries.iter().map(|(path, _)| path.clone()).collect(),
        ts: entries
            .into_iter()
            .map(|(path, components)| {
                (
                    path,
                    CheckFileFacts {
                        react: Some(FileAnalysis {
                            components,
                            dependencies: Vec::new(),
                        }),
                        ..CheckFileFacts::default()
                    },
                )
            })
            .collect::<HashMap<_, _>>(),
        stats: Default::default(),
    }
}

#[test]
fn run_analyze_inner_with_facts_uses_root_matches_and_cached_children() {
    let root = fixture("nested");
    let parent_path = root.join("app/components/Parent.tsx");
    let child_path = root.join("app/components/Child.tsx");
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
    ];
    let mut child = component("Child", "app/components/Child.tsx");
    child.fetches.push(FetchCall {
        file: child.file.clone(),
        exported_name: None,
        shape: None,
    });
    let shared = facts(vec![(parent_path, vec![parent]), (child_path, vec![child])]);
    let file_config = FileConfig {
        frontend_root: Some("app".to_string()),
        assert_no_fetch: None,
    };

    let results = run_analyze_inner_with_facts(
        &root,
        &file_config,
        &["app/components/Parent.tsx".to_string()],
        &shared,
    )
    .unwrap();

    let parent = results
        .iter()
        .find(|component| component.name == "Parent")
        .unwrap();
    assert!(parent
        .inherited_from_children
        .as_ref()
        .is_some_and(|agg| agg.has_fetch));
}

#[test]
fn run_analyze_inner_with_facts_uses_default_targets_and_exact_child_paths() {
    let root = fixture("nested");
    let parent_path = root.join("app/components/Parent.tsx");
    let child_path = root.join("app/components/Child.tsx");
    let mut parent = component("Parent", "app/components/Parent.tsx");
    parent.children = vec![ComponentRef {
        file: child_path.to_string_lossy().to_string(),
        name: "Child".to_string(),
    }];
    let mut child = component("Child", child_path.to_string_lossy().as_ref());
    child.has_state = true;
    let shared = facts(vec![(parent_path, vec![parent]), (child_path, vec![child])]);
    let file_config = FileConfig {
        frontend_root: Some("app".to_string()),
        assert_no_fetch: None,
    };

    let results = run_analyze_inner_with_facts(&root, &file_config, &[], &shared).unwrap();

    let parent = results
        .iter()
        .find(|component| component.name == "Parent")
        .unwrap();
    assert!(parent
        .inherited_from_children
        .as_ref()
        .is_some_and(|agg| agg.has_state));
}

#[test]
fn run_analyze_inner_with_facts_covers_fallback_missing_cache_and_errors() {
    let root = fixture("nested");
    let file_config = FileConfig {
        frontend_root: Some("app".to_string()),
        assert_no_fetch: None,
    };
    let shared = facts(Vec::new());

    let missing_cache = run_analyze_inner_with_facts(
        &root,
        &file_config,
        &["app/components/Child.tsx".to_string()],
        &shared,
    )
    .unwrap();
    assert!(missing_cache.is_empty());

    let fallback = run_analyze_inner_with_facts(
        &root,
        &file_config,
        &["components/Parent.tsx".to_string()],
        &shared,
    )
    .unwrap();
    assert!(fallback.is_empty());

    let missing_frontend = run_analyze_inner_with_facts(
        &root.join("missing"),
        &file_config,
        &["components/Parent.tsx".to_string()],
        &shared,
    );
    assert!(missing_frontend.is_err());

    let bad_file = root.join("app/components/Child.tsx");
    let mut bad_facts = facts(Vec::new());
    bad_facts.ts.insert(
        bad_file,
        CheckFileFacts {
            parse_error: Some("syntax error".to_string()),
            ..CheckFileFacts::default()
        },
    );
    let parse_error = run_analyze_inner_with_facts(
        &root,
        &file_config,
        &["app/components/Child.tsx".to_string()],
        &bad_facts,
    )
    .unwrap_err();
    assert!(parse_error.to_string().contains("syntax error"));
}
