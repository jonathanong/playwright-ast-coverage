use super::{print_results, print_violations};
use crate::react_traits::report::types::{
    AggregatedFacts, ComponentFacts, ComponentRef, Environment, FetchCall, Violation,
};

fn make_facts(name: &str, file: &str) -> ComponentFacts {
    ComponentFacts {
        name: name.to_string(),
        file: file.to_string(),
        environment: Environment::Unknown,
        has_state: false,
        has_props: false,
        passes_props: false,
        uses_memo: false,
        uses_context_provider: false,
        uses_suspense: false,
        fetches: vec![],
        dependencies: vec![],
        children: vec![],
        inherited_from_children: None,
    }
}

#[test]
fn print_results_no_depth_produces_output() {
    let facts = make_facts("default", "app/components/Greeting.tsx");
    print_results(&[facts], 0);
}

#[test]
fn print_results_with_depth_produces_output() {
    let mut facts = make_facts("default", "app/components/Parent.tsx");
    facts.children = vec![ComponentRef {
        name: "default".to_string(),
        file: "app/components/Child.tsx".to_string(),
    }];
    facts.dependencies = vec!["app/components/Child.tsx".to_string()];
    print_results(&[facts], 1);
}

#[test]
fn print_results_with_inherited_from_children() {
    let mut facts = make_facts("default", "app/components/Parent.tsx");
    facts.inherited_from_children = Some(AggregatedFacts {
        has_fetch: true,
        ..Default::default()
    });
    print_results(&[facts], 0);
}

#[test]
fn print_results_with_various_environments() {
    let mut server = make_facts("default", "app/server.tsx");
    server.environment = Environment::Server;
    let mut client = make_facts("default", "app/client.tsx");
    client.environment = Environment::Client;
    let mut shared = make_facts("default", "app/shared.tsx");
    shared.environment = Environment::Shared;
    print_results(&[server, client, shared], 0);
}

#[test]
fn print_results_with_fetches() {
    let mut facts = make_facts("default", "app/components/Fetcher.tsx");
    facts.fetches = vec![FetchCall {
        file: "app/components/Fetcher.tsx".to_string(),
        exported_name: None,
        shape: Some("GET /api/users".to_string()),
    }];
    print_results(&[facts], 0);
}

#[test]
fn print_violations_outputs_violations() {
    let violations = vec![Violation {
        component: "Fetcher".to_string(),
        file: "app/components/Fetcher.tsx".to_string(),
        rule: "assert-no-fetch".to_string(),
        detail: Some("GET /api/users".to_string()),
    }];
    print_violations(&violations);
}

#[test]
fn print_violations_no_detail() {
    let violations = vec![Violation {
        component: "Fetcher".to_string(),
        file: "app/components/Fetcher.tsx".to_string(),
        rule: "assert-no-fetch".to_string(),
        detail: None,
    }];
    print_violations(&violations);
}
