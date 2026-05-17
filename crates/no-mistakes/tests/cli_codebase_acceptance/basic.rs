use super::common::{
    assert_success, file_paths, fixture, run, run_in, run_json, stdout, via_kinds,
};
use serde_json::Value;

#[test]
fn dependencies_acceptance_basic_cli_behaviors() {
    let root = fixture("simple");
    let root_arg = root.to_string_lossy();

    let output = run_in(
        &root,
        &[
            "dependencies",
            "--root",
            root_arg.as_ref(),
            "--format",
            "json",
            "a.mts",
        ],
    );
    assert_success(&output);
    assert_eq!(
        file_paths(&serde_json::from_str(&stdout(&output)).unwrap()),
        vec!["b.mts", "c.mts"]
    );

    for args in [
        vec!["dependencies", "--root", root_arg.as_ref(), "a.mts"],
        vec![
            "dependencies",
            "--root",
            root_arg.as_ref(),
            "--json",
            "a.mts",
        ],
    ] {
        let output = run_in(&root, &args);
        assert_success(&output);
        assert!(serde_json::from_str::<Value>(&stdout(&output)).unwrap()["files"].is_array());
    }

    let yml = run_in(
        &root,
        &[
            "dependencies",
            "--root",
            root_arg.as_ref(),
            "--format",
            "yml",
            "a.mts",
        ],
    );
    assert_success(&yml);
    assert!(stdout(&yml).contains("files:"));

    let human = run_in(
        &root,
        &[
            "dependencies",
            "--root",
            root_arg.as_ref(),
            "--format",
            "human",
            "a.mts",
        ],
    );
    assert_success(&human);
    assert!(stdout(&human).contains("a.mts") && stdout(&human).contains("b.mts"));

    let multiple = run_json(&root, &["dependencies", "a.mts", "b.mts"]);
    assert!(file_paths(&multiple).contains(&"c.mts".to_string()));

    let depth = run_json(&root, &["dependencies", "--depth", "1", "a.mts"]);
    for file in depth["files"].as_array().expect("files array") {
        assert!(file["depth"].as_u64().unwrap() <= 1);
    }

    let paths = run_in(
        &root,
        &[
            "dependencies",
            "--root",
            root_arg.as_ref(),
            "--format",
            "paths",
            "a.mts",
        ],
    );
    assert_success(&paths);
    assert!(stdout(&paths).lines().any(|line| line == "b.mts"));

    let missing = run_in(
        &root,
        &[
            "dependencies",
            "--root",
            root_arg.as_ref(),
            "--format",
            "json",
            "nonexistent_xyz.mts",
        ],
    );
    assert_success(&missing);
    assert!(
        serde_json::from_str::<Value>(&stdout(&missing)).unwrap()["files"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    let relative_root = run(&[
        "dependencies",
        "--root",
        root_arg.as_ref(),
        "--format",
        "json",
        "a.mts",
    ]);
    assert_success(&relative_root);
    assert!(!file_paths(&serde_json::from_str(&stdout(&relative_root)).unwrap()).is_empty());

    let invalid = run(&[
        "dependencies",
        "--root",
        root_arg.as_ref(),
        "--relationship",
        "typo",
        "a.mts",
    ]);
    assert!(!invalid.status.success());

    let serial = run_json(&root, &["-j", "1", "dependencies", "a.mts"]);
    let parallel = run_json(&root, &["-j", "8", "dependencies", "a.mts"]);
    assert_eq!(parallel, serial);
}

#[test]
fn dependents_acceptance_basic_cli_behaviors() {
    let root = fixture("dependents-basic");
    let root_arg = root.to_string_lossy();

    for args in [
        vec![
            "dependents",
            "--root",
            root_arg.as_ref(),
            "--relationship",
            "import",
            "src/source.mts",
        ],
        vec![
            "dependents",
            "--root",
            root_arg.as_ref(),
            "--json",
            "--relationship",
            "import",
            "src/source.mts",
        ],
    ] {
        let output = run(&args);
        assert_success(&output);
        assert!(serde_json::from_str::<Value>(&stdout(&output)).unwrap()["files"].is_array());
    }

    let yml = run(&[
        "dependents",
        "--root",
        root_arg.as_ref(),
        "--format",
        "yml",
        "--relationship",
        "import",
        "src/source.mts",
    ]);
    assert_success(&yml);
    assert!(stdout(&yml).contains("files:"));

    let human = run(&[
        "dependents",
        "--root",
        root_arg.as_ref(),
        "--format",
        "human",
        "--relationship",
        "import",
        "src/source.mts",
    ]);
    assert_success(&human);
    assert!(stdout(&human).contains("src/source.mts") && stdout(&human).contains("src/mid.mts"));

    let multiple = run_json(
        &root,
        &["dependents", "src/source.mts", "scripts/child.mts"],
    );
    let multiple_paths = file_paths(&multiple);
    assert!(multiple_paths.contains(&"src/mid.mts".to_string()));
    assert!(multiple_paths.contains(&"scripts/runner.mts".to_string()));

    let depth = run_json(
        &root,
        &[
            "dependents",
            "--relationship",
            "import",
            "--depth",
            "1",
            "src/source.mts",
        ],
    );
    let depth_paths = file_paths(&depth);
    assert!(depth_paths.contains(&"src/mid.mts".to_string()));
    assert!(!depth_paths.contains(&"src/top.mts".to_string()));

    let paths = run(&[
        "dependents",
        "--root",
        root_arg.as_ref(),
        "--format",
        "paths",
        "--relationship",
        "import",
        "src/source.mts",
    ]);
    assert_success(&paths);
    assert_eq!(
        stdout(&paths).lines().collect::<Vec<_>>(),
        vec!["src/mid.mts", "src/top.mts"]
    );

    let missing = run_in(
        &root,
        &[
            "dependents",
            "--root",
            root_arg.as_ref(),
            "--format",
            "json",
            "nonexistent_xyz_97.mts",
        ],
    );
    assert_success(&missing);
    assert!(
        serde_json::from_str::<Value>(&stdout(&missing)).unwrap()["files"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    let relative = run_json(
        &root,
        &["dependents", "--relationship", "import", "src/source.mts"],
    );
    assert!(file_paths(&relative).contains(&"src/mid.mts".to_string()));

    let md = run_json(
        &root,
        &["dependents", "--relationship", "md", "src/source.mts"],
    );
    assert_eq!(file_paths(&md), vec!["docs/source-link.md"]);
    assert_eq!(via_kinds(&md, "docs/source-link.md"), vec!["md"]);

    let test = run_json(
        &root,
        &["dependents", "--relationship", "test", "src/source.mts"],
    );
    assert_eq!(file_paths(&test), vec!["src/source.test.mts"]);
    assert_eq!(via_kinds(&test, "src/source.test.mts"), vec!["test"]);

    let process = run_json(
        &root,
        &[
            "dependents",
            "--relationship",
            "process",
            "scripts/child.mts",
        ],
    );
    assert_eq!(file_paths(&process), vec!["scripts/runner.mts"]);
    assert_eq!(via_kinds(&process, "scripts/runner.mts"), vec!["process"]);
}

#[test]
fn symbols_acceptance_cli_behaviors() {
    let root = fixture("simple");
    let root_arg = root.to_string_lossy();

    let help = run(&["symbols", "--help"]);
    assert_success(&help);
    let help = stdout(&help);
    assert!(help.contains("FILE"));
    assert!(help.contains("--include"));
    assert!(help.contains("--kind"));

    let missing_file_arg = run(&["symbols"]);
    assert!(!missing_file_arg.status.success());

    let default_json = run(&["symbols", "--root", root_arg.as_ref(), "a.mts"]);
    assert_success(&default_json);
    let value: Value = serde_json::from_str(&stdout(&default_json)).unwrap();
    assert_eq!(value["roots"][0], "a.mts");
    assert_eq!(value["files"][0]["exports"][0]["name"], "a");
    assert!(value["files"][0].get("imports").is_none());

    let include_both = run(&[
        "symbols",
        "--root",
        root_arg.as_ref(),
        "--include",
        "both",
        "--format",
        "json",
        "a.mts",
    ]);
    assert_success(&include_both);
    let value: Value = serde_json::from_str(&stdout(&include_both)).unwrap();
    let import = &value["files"][0]["imports"][0];
    assert_eq!(import["imported"], "b");
    assert_eq!(import["source"], "./b.mts");
    assert_eq!(import["resolved"], "b.mts");

    let invalid_kind = run(&[
        "symbols",
        "--root",
        root_arg.as_ref(),
        "--kind",
        "functoin",
        "a.mts",
    ]);
    assert!(!invalid_kind.status.success());

    let paths = run(&[
        "symbols",
        "--root",
        root_arg.as_ref(),
        "--format",
        "paths",
        "a.mts",
    ]);
    assert_success(&paths);
    let lines = stdout(&paths)
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("a.mts:"));
    assert!(lines[0].ends_with(":a"));
}

#[test]
fn top_level_version_flag_prints_version() {
    let output = run(&["--version"]);
    assert_success(&output);
    let stdout = stdout(&output);
    let parts = stdout.split_whitespace().collect::<Vec<_>>();
    assert_eq!(parts.first(), Some(&"no-mistakes"));
    assert!(parts.get(1).is_some_and(|version| version.contains('.')));
}
