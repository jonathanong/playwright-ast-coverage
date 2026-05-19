use std::path::PathBuf;
use std::process::{Command, Output};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_no-mistakes"))
}

fn fixture(category: &str, name: &str) -> PathBuf {
    no_mistakes_core::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(category)
            .join(name),
    )
}

fn run(args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("no-mistakes should run")
}

fn run_in(dir: &std::path::Path, args: &[&str]) -> Output {
    Command::new(bin())
        .current_dir(dir)
        .args(args)
        .output()
        .expect("no-mistakes should run")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be utf8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be utf8")
}

#[test]
fn dependencies_cli_covers_relative_roots_cwd_root_and_absolute_entrypoint() {
    let workspace = fixture("codebase-analysis", "simple")
        .parent()
        .expect("fixture parent")
        .to_path_buf();
    let relative_root = run_in(
        &workspace,
        &[
            "dependencies",
            "a.mts",
            "--root",
            "simple",
            "--format",
            "json",
        ],
    );
    assert!(relative_root.status.success());

    let root = fixture("codebase-analysis", "simple");
    let cwd_root = run_in(&root, &["dependencies", "a.mts", "--format", "json"]);
    assert!(cwd_root.status.success());

    let absolute = root.join("a.mts");
    let absolute_file = run(&[
        "dependents",
        absolute.to_str().unwrap(),
        "--root",
        root.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(absolute_file.status.success());
}

#[test]
fn invalid_global_filter_surfaces_main_error_exit() {
    let root = fixture("server-ast-routes", "express");
    let output = run(&[
        "server",
        "--root",
        root.to_str().unwrap(),
        "--filter",
        "[",
        "routes",
    ]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("error:"));
}

#[test]
fn wrapper_help_exposes_parity_flags() {
    let queues = run(&["queues", "--help"]);
    assert!(queues.status.success());
    let queues_help = stdout(&queues);
    assert!(queues_help.contains("--depth"));
    assert!(queues_help.contains("--timings"));

    let server = run(&["server", "--help"]);
    assert!(server.status.success());
    let server_help = stdout(&server);
    assert!(server_help.contains("--tsconfig"));
    assert!(server_help.contains("--depth"));
    assert!(server_help.contains("--timings"));

    let check = run(&["check", "--help"]);
    assert!(check.status.success());
    assert!(stdout(&check).contains("--json"));
}

#[test]
fn dependencies_cli_covers_symbol_error_test_glob_and_timing_paths() {
    let root = fixture("codebase-analysis", "simple");

    let symbol_in_deps = run(&[
        "dependencies",
        "a.mts#a",
        "--root",
        root.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(symbol_in_deps.status.code(), Some(2));

    let with_test_glob_and_timings = run(&[
        "dependencies",
        "a.mts",
        "--root",
        root.to_str().unwrap(),
        "--test",
        "cargo",
        "--timings",
        "--json",
    ]);
    assert!(with_test_glob_and_timings.status.success());
    assert!(stderr(&with_test_glob_and_timings).contains("search:"));

    let invalid_filter = run(&[
        "dependencies",
        "a.mts",
        "--root",
        root.to_str().unwrap(),
        "--filter",
        "[",
        "--json",
    ]);
    assert_eq!(invalid_filter.status.code(), Some(2));

    let missing_tsconfig = run(&[
        "dependencies",
        "a.mts",
        "--root",
        root.to_str().unwrap(),
        "--tsconfig",
        "missing-tsconfig.json",
        "--json",
    ]);
    assert_eq!(missing_tsconfig.status.code(), Some(2));
}

#[test]
fn dependents_cli_covers_mixed_symbol_and_plain_entrypoints() {
    let root = fixture("codebase-analysis", "symbol-export");
    let output = run(&[
        "dependents",
        "source.mts#alpha",
        "source.mts",
        "--root",
        root.to_str().unwrap(),
        "--json",
    ]);

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    let paths: Vec<_> = json["files"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|file| file["path"].as_str())
        .collect();
    assert!(paths.contains(&"uses-alpha.mts"));
}

#[test]
fn dependencies_cli_covers_process_spawn_walk_fixture() {
    let root = fixture("ast-snippets", "ts-process-spawn/project");
    let output = run(&[
        "dependencies",
        "configs/spawn-all.tsx",
        "--root",
        root.to_str().unwrap(),
        "--relationship",
        "process",
        "--format",
        "json",
    ]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("scripts/root.mts"));
}

#[test]
fn symbols_without_root_uses_cwd_as_root() {
    // Exercises the None branch in symbols::resolve_root (no --root flag).
    let root = fixture("codebase-analysis", "simple");
    let output = run_in(&root, &["symbols", "a.mts", "--json"]);
    assert!(output.status.success());
    let json: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("valid json from symbols");
    assert!(json["files"].as_array().is_some());
}

#[test]
fn symbols_with_relative_root_resolves_against_cwd() {
    // Exercises the Some(relative_path) branch in symbols::resolve_root.
    let cwd = fixture("codebase-analysis", "simple")
        .parent()
        .unwrap()
        .to_path_buf();
    let output = run_in(&cwd, &["symbols", "--root", "simple", "a.mts", "--json"]);
    assert!(output.status.success());
    let json: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("valid json from symbols");
    assert!(json["files"].as_array().is_some());
}

#[test]
fn dependencies_with_unknown_test_framework_returns_unfiltered_results() {
    // Exercises the `_ => vec![]` branch in dependencies::test_globs
    // and the `RelationshipArg::All => return None` branch in relationship_filter.
    let root = fixture("codebase-analysis", "simple");
    let output = run(&[
        "dependencies",
        "a.mts",
        "--root",
        root.to_str().unwrap(),
        "--test",
        "unknown-framework",
        "--json",
    ]);
    assert!(output.status.success());

    let all_relationships = run(&[
        "dependencies",
        "a.mts",
        "--root",
        root.to_str().unwrap(),
        "--relationship",
        "all",
        "--json",
    ]);
    assert!(all_relationships.status.success());
}

#[test]
fn react_run_check_in_process_exercises_run_check_function() {
    // Exercises no_mistakes_core::react_traits::run_check directly so the
    // in-process test binary instantiation covers that function and the
    // assert_no_fetch_violations helper.
    let root = fixture("react-traits-config", "assert-no-fetch");
    let violations =
        no_mistakes_core::react_traits::run_check(&root, None, &[], true).expect("run_check ok");
    assert!(
        !violations.is_empty(),
        "expected violations with assert_no_fetch=true"
    );

    // Also exercise check_enabled to cover that instantiation.
    let enabled =
        no_mistakes_core::react_traits::check_enabled(&root, None, true).expect("check_enabled ok");
    assert!(enabled);

    // Trigger error paths (config not found) to cover the `?` error branches
    // in this binary instantiation.
    let bad_config = std::path::Path::new("nonexistent-config.yaml");
    assert!(
        no_mistakes_core::react_traits::run_check(&root, Some(bad_config), &[], false).is_err()
    );
    assert!(no_mistakes_core::react_traits::check_enabled(&root, Some(bad_config), false).is_err());

    // Exercise run_check_with_facts error path.
    let facts = no_mistakes_core::codebase::check_facts::CheckFactMap::default();
    assert!(no_mistakes_core::react_traits::run_check_with_facts(
        &root,
        Some(bad_config),
        &[],
        false,
        &facts
    )
    .is_err());

    // Exercise the early-return path (effective_no_fetch=false) in this binary instantiation.
    // Uses a root with no assert_no_fetch config and assert_no_fetch=false.
    let no_fetch_root = fixture("react-traits-components", "nested");
    let empty = no_mistakes_core::react_traits::run_check(&no_fetch_root, None, &[], false)
        .expect("run_check with no-fetch disabled should succeed");
    assert!(
        empty.is_empty(),
        "expected no violations when assert_no_fetch=false"
    );

    // Also exercise run_check_with_facts early-return path.
    let empty_facts = no_mistakes_core::codebase::check_facts::CheckFactMap::default();
    let empty2 = no_mistakes_core::react_traits::run_check_with_facts(
        &no_fetch_root,
        None,
        &[],
        false,
        &empty_facts,
    )
    .expect("run_check_with_facts with no-fetch disabled should succeed");
    assert!(
        empty2.is_empty(),
        "expected no violations when assert_no_fetch=false"
    );
}

#[test]
fn react_check_with_nonexistent_config_path_returns_error() {
    // Exercises the error path for `?` on load_config in run_check and check_enabled
    // within the subprocess binary instantiation of no_mistakes_core.
    let root = fixture("react-traits-config", "assert-no-fetch");
    let output = run(&[
        "react",
        "--root",
        root.to_str().unwrap(),
        "--config",
        "/nonexistent/config.yaml",
        "check",
        "--assert-no-fetch",
    ]);
    assert_ne!(output.status.code(), Some(0));
}

#[test]
fn symbols_with_explicit_tsconfig_path() {
    // Exercises the Some(path) branch in symbols::resolve_tsconfig (--tsconfig flag).
    let root = fixture("codebase-analysis", "aliased");
    let tsconfig = root.join("tsconfig.json");
    let output = run(&[
        "symbols",
        "main.mts",
        "--root",
        root.to_str().unwrap(),
        "--tsconfig",
        tsconfig.to_str().unwrap(),
        "--json",
    ]);
    assert!(output.status.success());
    let json: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("valid json from symbols");
    assert!(json["files"].as_array().is_some());
}
