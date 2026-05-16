use super::*;

// ── extract_binary_names ─────────────────────────────────────────────

#[test]
fn cargo_run_bin() {
    let names = extract_binary_names("cargo run --bin guardrails -- --help");
    assert_eq!(names, vec!["guardrails"]);
}

#[test]
fn cargo_run_bin_hyphenated() {
    let names = extract_binary_names("cargo run --bin pg-schema -- --help");
    assert_eq!(names, vec!["pg-schema"]);
}

#[test]
fn cargo_run_p() {
    let names = extract_binary_names("cargo run -p my-tool -- serve");
    assert_eq!(names, vec!["my-tool"]);
}

#[test]
fn target_binary() {
    let names = extract_binary_names("./target/release/dependencies --help");
    assert_eq!(names, vec!["dependencies"]);
}

#[test]
fn no_binary_run() {
    let names = extract_binary_names("echo hello && ls -la");
    assert!(names.is_empty());
}

#[test]
fn deduplicates_same_binary() {
    let names =
        extract_binary_names("cargo run --bin foo -- step1 && cargo run --bin foo -- step2");
    assert_eq!(names, vec!["foo"]);
}

#[test]
fn multiple_binaries_in_one_run() {
    let run = "cargo run --bin api --help\ncargo run --bin web --help";
    let names = extract_binary_names(run);
    assert!(names.contains(&"api".to_string()));
    assert!(names.contains(&"web".to_string()));
}

// ── extract_invocations ──────────────────────────────────────────────

#[test]
fn extracts_run_step() {
    let yaml = r#"
jobs:
  build:
    steps:
      - name: Test
        run: cargo run --bin guardrails -- --help
"#;
    let invocations = extract_invocations(yaml).unwrap();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].step_name.as_deref(), Some("Test"));
    assert!(invocations[0].binaries.contains(&"guardrails".to_string()));
}

#[test]
fn extracts_uses_step() {
    let yaml = r#"
jobs:
  build:
    steps:
      - uses: actions/checkout@v4
"#;
    let invocations = extract_invocations(yaml).unwrap();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].run, "actions/checkout@v4");
    assert!(invocations[0].binaries.is_empty());
}

#[test]
fn multiple_steps() {
    let yaml = r#"
jobs:
  ci:
    steps:
      - run: cargo run --bin pg-schema -- --help
      - name: Run guardrails
        run: cargo run --bin guardrails -- --help
"#;
    let invocations = extract_invocations(yaml).unwrap();
    assert_eq!(invocations.len(), 2);
    assert!(invocations[0].binaries.contains(&"pg-schema".to_string()));
    assert!(invocations[1].binaries.contains(&"guardrails".to_string()));
}

#[test]
fn empty_yaml_returns_empty() {
    let invocations = extract_invocations("{}").unwrap();
    assert!(invocations.is_empty());
}

#[test]
fn step_without_name_or_run() {
    let yaml = r#"
jobs:
  build:
    steps:
      - if: ${{ github.event_name == 'push' }}
        run: echo hi
"#;
    let invocations = extract_invocations(yaml).unwrap();
    assert_eq!(invocations.len(), 1);
    assert!(invocations[0].step_name.is_none());
}

// ── parse_cargo_bins ─────────────────────────────────────────────────

#[test]
fn parses_bin_entries() {
    let toml_str = r#"
[package]
name = "exec"

[[bin]]
name = "guardrails"
path = "src/bin/guardrails.rs"

[[bin]]
name = "pg-schema"
path = "src/bin/pg_schema.rs"
"#;
    let bins = parse_cargo_bins(toml_str).unwrap();
    assert_eq!(
        bins.get("guardrails").map(String::as_str),
        Some("src/bin/guardrails.rs")
    );
    assert_eq!(
        bins.get("pg-schema").map(String::as_str),
        Some("src/bin/pg_schema.rs")
    );
}

#[test]
fn no_bin_entries_returns_empty() {
    let toml_str = r#"[package]
name = "exec"
version = "0.1.0"
"#;
    let bins = parse_cargo_bins(toml_str).unwrap();
    assert!(bins.is_empty());
}
