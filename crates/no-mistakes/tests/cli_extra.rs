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
fn react_check_targeted_run_skips_disabled_assert_no_fetch() {
    let root = fixture("react-traits-components", "nested");
    let output = run(&[
        "react",
        "--root",
        root.to_str().unwrap(),
        "check",
        "app/components/Child.tsx",
    ]);

    assert!(output.status.success());
    assert!(stdout(&output).is_empty());
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
fn global_check_skips_unconfigured_expensive_checks() {
    let root = fixture("codebase-analysis", "check-configured-only");
    let output = run(&["check", "--root", root.to_str().unwrap(), "--json"]);

    assert!(output.status.success());
    assert!(
        stderr(&output).is_empty(),
        "unconfigured checks should not warn: {}",
        stderr(&output)
    );
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    for key in ["react", "queues", "rules", "integration", "codebase"] {
        assert_eq!(
            json[key].as_array().map(Vec::len),
            Some(0),
            "{key} should be empty in {json}"
        );
    }
}

#[test]
fn global_check_timings_are_reported_in_phase_order() {
    let root = fixture("codebase-analysis", "check-configured-only");
    let output = run(&[
        "check",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "json",
        "--timings",
    ]);

    assert!(output.status.success());
    let err = stderr(&output);
    let mut previous = 0;
    for label in ["react:", "queues:", "rules:", "integration:", "codebase:"] {
        let index = err
            .find(label)
            .unwrap_or_else(|| panic!("missing {label} in {err}"));
        assert!(index >= previous, "{label} should be in phase order: {err}");
        previous = index;
    }
}

#[test]
fn global_check_reports_integration_suite_findings() {
    let root = fixture("integration-tests", "basic");
    let json_output = run(&["check", "--root", root.to_str().unwrap(), "--json"]);
    assert_eq!(json_output.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_str(&stdout(&json_output)).unwrap();
    let integration = json["integration"].as_array().unwrap();
    assert!(integration.iter().any(|finding| {
        finding["file"] == "backend/unit.test.mts"
            && finding["testName"] == "helper integration in unit suite"
            && finding["integration"] == "openai"
    }));

    let paths = run(&[
        "check",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "paths",
    ]);
    assert_eq!(paths.status.code(), Some(1));
    assert!(stdout(&paths).contains("backend/unit.test.mts:"));

    let human = run(&[
        "check",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "human",
    ]);
    assert_eq!(human.status.code(), Some(1));
    assert!(stdout(&human).contains("integration[vitest:unit]"));

    let markdown = run(&["check", "--root", root.to_str().unwrap(), "--format", "md"]);
    assert_eq!(markdown.status.code(), Some(1));
    assert!(stdout(&markdown).contains("vitest suite unit does not allow integration tests"));
}

#[test]
fn global_check_fails_when_config_is_invalid() {
    let root = fixture("react-traits-config", "invalid");
    let output = run(&[
        "check",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "human",
    ]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("failed to parse"));
}

#[test]
fn global_check_warns_when_enabled_react_scan_fails() {
    let root = fixture("react-traits-config", "assert-no-fetch-invalid");
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
fn global_check_warns_when_react_shared_facts_check_errors() {
    let root = fixture("react-traits-config", "assert-no-fetch-missing-frontend");
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
fn global_check_skips_disabled_unique_exports_rule() {
    let root = fixture("codebase-analysis", "unique-exports-config-disabled");
    let output = run(&["check", "--root", root.to_str().unwrap(), "--json"]);

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(json["codebase"].as_array().map(Vec::len), Some(0));
}

#[test]
fn global_check_surfaces_unique_exports_config_errors() {
    let root = fixture("codebase-analysis", "unique-exports-invalid-skip");
    let output = run(&["check", "--root", root.to_str().unwrap()]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("invalid skip file pattern"));
}

#[test]
fn global_check_runs_test_no_unmocked_dynamic_imports_rule() {
    let root = fixture("codebase-analysis", "test-no-unmocked-dynamic-imports");
    let json = run(&["check", "--root", root.to_str().unwrap(), "--json"]);
    assert_eq!(json.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_str(&stdout(&json)).unwrap();
    let rules = value["rules"].as_array().unwrap();
    assert!(rules.iter().any(|finding| {
        finding["target"].as_str() == Some("src/unmocked-child.mts")
            && finding["rule"].as_str() == Some("test-no-unmocked-dynamic-imports")
    }));

    let paths = run(&[
        "check",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "paths",
    ]);
    assert_eq!(paths.status.code(), Some(1));
    assert!(stdout(&paths).contains("tests/bad.test.mts"));

    for format in ["human", "md"] {
        let output = run(&[
            "check",
            "--root",
            root.to_str().unwrap(),
            "--format",
            format,
        ]);
        assert_eq!(output.status.code(), Some(1));
        assert!(stdout(&output).contains("test-no-unmocked-dynamic-imports"));
    }
}
