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
fn react_analyze_paths_outputs_component_files() {
    let root = fixture("react-traits-components", "basic");
    let output = run(&[
        "react",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "paths",
        "analyze",
        "app/components/Greeting.tsx",
    ]);

    assert!(output.status.success());
    assert!(stdout(&output).contains("Greeting.tsx"));
}

#[test]
fn react_check_formats_report_violations() {
    let root = fixture("react-traits-config", "assert-no-fetch");
    for format in ["json", "yml", "md", "paths", "human"] {
        let output = run(&[
            "react",
            "--root",
            root.to_str().unwrap(),
            "--format",
            format,
            "check",
            "--assert-no-fetch",
            "app/components/Fetcher.tsx",
        ]);

        assert_eq!(output.status.code(), Some(1));
        assert!(stdout(&output).contains("Fetcher.tsx"));
    }
}

#[test]
fn react_analyze_markdown_and_yaml_formats_render() {
    let root = fixture("react-traits-components", "basic");
    for format in ["md", "yml"] {
        let output = run(&[
            "react",
            "--root",
            root.to_str().unwrap(),
            "--format",
            format,
            "analyze",
            "app/components/Greeting.tsx",
        ]);
        assert!(output.status.success());
        assert!(stdout(&output).contains("Greeting"));
    }
}

#[test]
fn queue_and_server_markdown_and_yaml_formats_render() {
    let queue_basic = fixture("queue-ast-hop", "basic");
    let queue_dynamic = fixture("queue-ast-hop", "dynamic");
    let server = fixture("server-ast-routes", "express");
    let cases = [
        (
            &queue_basic,
            "queues",
            &["edges"][..],
            "enqueue.ts",
            Some(0),
        ),
        (
            &queue_basic,
            "queues",
            &["related", "enqueue.ts"],
            "worker.ts",
            Some(0),
        ),
        (&queue_dynamic, "queues", &["check"], "queues.ts", Some(1)),
        (&server, "server", &["routes"], "users", Some(0)),
        (&server, "server", &["edges"], "users", Some(0)),
        (
            &server,
            "server",
            &["related", "backend/api/users.ts"],
            "users",
            Some(0),
        ),
    ];
    for (root, tool, rest, needle, code) in cases {
        for format in ["md", "yml"] {
            let mut args = vec![tool, "--root", root.to_str().unwrap(), "--format", format];
            args.extend_from_slice(rest);
            let output = run(&args);
            assert_eq!(output.status.code(), code);
            let out = stdout(&output);
            assert!(out.contains(needle), "missing {needle} in {out}");
        }
    }
}

#[test]
fn global_check_formats_cover_clean_and_failing_projects() {
    let clean = fixture("queue-ast-hop", "basic");
    let json_alias = run(&["check", "--root", clean.to_str().unwrap(), "--json"]);
    assert!(json_alias.status.success());
    assert!(stdout(&json_alias).contains("queues"));

    for format in ["json", "md", "yml"] {
        let output = run(&[
            "check",
            "--root",
            clean.to_str().unwrap(),
            "--format",
            format,
        ]);
        assert!(output.status.success());
        assert!(stdout(&output).contains("queues"));
    }

    let failing = fixture("queue-ast-hop", "dynamic");
    for format in ["paths", "human"] {
        let output = run(&[
            "check",
            "--root",
            failing.to_str().unwrap(),
            "--format",
            format,
        ]);

        assert_eq!(output.status.code(), Some(1));
        assert!(stdout(&output).contains("queues.ts"));
    }
    let failing_md = run(&[
        "check",
        "--root",
        failing.to_str().unwrap(),
        "--format",
        "md",
    ]);
    assert_eq!(failing_md.status.code(), Some(1));
    assert!(stdout(&failing_md).contains("queues.ts"));

    let react_failing = fixture("react-traits-config", "assert-no-fetch");
    for format in ["paths", "human"] {
        let output = run(&[
            "check",
            "--root",
            react_failing.to_str().unwrap(),
            "--format",
            format,
        ]);

        assert_eq!(output.status.code(), Some(1));
        assert!(stdout(&output).contains("Fetcher.tsx"));
    }
}

#[test]
fn global_check_warns_when_react_config_is_invalid() {
    let root = fixture("react-traits-config", "invalid");
    let output = run(&[
        "check",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "human",
    ]);

    assert!(output.status.success());
    assert!(stderr(&output).contains("warning: react check skipped:"));
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
    assert!(String::from_utf8(output.stderr).unwrap().contains("error:"));
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
