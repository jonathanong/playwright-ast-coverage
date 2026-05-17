use super::analyze_file;
use std::path::PathBuf;

fn fixture(category: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(category)
        .join(name)
}

#[test]
fn analyze_basic_greeting() {
    let root = fixture("react-traits-components", "basic");
    let file = root.join("app/components/Greeting.tsx");
    let result = analyze_file(&file, &root).expect("should succeed");
    assert_eq!(result.components.len(), 1);
    assert_eq!(result.components[0].name, "default");
}

#[test]
fn analyze_counter_has_state() {
    let root = fixture("react-traits-components", "basic");
    let file = root.join("app/components/Counter.tsx");
    let result = analyze_file(&file, &root).expect("should succeed");
    assert!(result.components[0].has_state);
}

#[test]
fn nonexistent_file_returns_error() {
    let root = fixture("react-traits-components", "basic");
    let file = root.join("app/components/DoesNotExist.tsx");
    assert!(analyze_file(&file, &root).is_err());
}

#[test]
fn invalid_tsx_returns_error() {
    let root = fixture("react-traits-analyze", "file-error");
    let file = root.join("invalid.tsx");
    assert!(analyze_file(&file, &root).is_err());
}

#[test]
fn analyze_server_component_environment() {
    use crate::react_traits::report::types::Environment;
    let root = fixture("react-traits-analyze", "environments");
    let file = root.join("ServerComp.tsx");
    let result = analyze_file(&file, &root).expect("should succeed");
    assert_eq!(result.components[0].environment, Environment::Server);
}

#[test]
fn analyze_client_component_environment() {
    use crate::react_traits::report::types::Environment;
    let root = fixture("react-traits-analyze", "environments");
    let file = root.join("ClientComp.tsx");
    let result = analyze_file(&file, &root).expect("should succeed");
    assert_eq!(result.components[0].environment, Environment::Client);
}

#[test]
fn multi_component_scopes_fetch_to_component_span() {
    // Two components in one file: FetchingComponent has fetch, PureComponent does not.
    // The FetchVisitor's in_scope = false path is exercised for calls outside each span.
    let root = fixture("react-traits-analyze", "multi-component");
    let file = root.join("app/components/Mixed.tsx");
    let analysis = analyze_file(&file, &root).expect("should analyze");

    let fetching = analysis
        .components
        .iter()
        .find(|c| c.name == "FetchingComponent")
        .expect("FetchingComponent not found");
    let pure = analysis
        .components
        .iter()
        .find(|c| c.name == "PureComponent")
        .expect("PureComponent not found");

    assert!(
        !fetching.fetches.is_empty(),
        "FetchingComponent should detect fetch"
    );
    assert!(
        pure.fetches.is_empty(),
        "PureComponent should not inherit FetchingComponent's fetch"
    );
}
