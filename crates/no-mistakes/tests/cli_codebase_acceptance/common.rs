use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_no-mistakes"))
}

pub fn fixture(name: &str) -> PathBuf {
    no_mistakes_core::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis")
            .join(name),
    )
}

pub fn run(args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("no-mistakes should run")
}

pub fn run_in(root: &Path, args: &[&str]) -> Output {
    Command::new(bin())
        .current_dir(root)
        .args(args)
        .output()
        .expect("no-mistakes should run")
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be utf8")
}

pub fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "exit code: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn run_json(root: &Path, args: &[&str]) -> Value {
    let root_arg = root.to_string_lossy();
    let mut command_args = Vec::from(args);
    command_args.extend(["--root", root_arg.as_ref(), "--format", "json"]);
    let output = run(&command_args);
    assert_success(&output);
    serde_json::from_str(&stdout(&output)).unwrap_or_else(|e| {
        panic!("invalid JSON: {e}\nstdout: {}", stdout(&output));
    })
}

pub fn file_paths(value: &Value) -> Vec<String> {
    value["files"]
        .as_array()
        .map(|files| {
            files
                .iter()
                .filter_map(|file| file["path"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

pub fn via_kinds(value: &Value, path: &str) -> Vec<String> {
    value["files"]
        .as_array()
        .and_then(|files| {
            files
                .iter()
                .find(|file| file["path"].as_str() == Some(path))
        })
        .and_then(|file| file["via"].as_array())
        .map(|kinds| {
            kinds
                .iter()
                .filter_map(|kind| kind.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

pub fn has_path_with_via(value: &Value, path: &str, via: &str) -> bool {
    value["files"].as_array().into_iter().flatten().any(|file| {
        file["path"].as_str() == Some(path)
            && file["via"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind.as_str() == Some(via)))
    })
}

pub fn has_queue_job_with_via(value: &Value, queue_file: &str, job: &str, via: &str) -> bool {
    value["files"].as_array().into_iter().flatten().any(|file| {
        file["queueFile"].as_str() == Some(queue_file)
            && file["job"].as_str() == Some(job)
            && file["via"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind.as_str() == Some(via)))
    })
}
