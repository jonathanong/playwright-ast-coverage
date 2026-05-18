use std::path::PathBuf;
use std::process::{Command, Output};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_no-mistakes"))
}

fn fixture(name: &str) -> PathBuf {
    no_mistakes_core::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis")
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

#[test]
fn global_check_scans_unique_exports_project_roots_outside_root() {
    let root = fixture("unique-exports-outside-root/app");
    let output = run(&["check", "--root", root.to_str().unwrap(), "--json"]);

    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    let codebase = json["codebase"].as_array().unwrap();
    assert!(codebase
        .iter()
        .any(|finding| finding["exportName"] == "OutsideDuplicate"));
}
