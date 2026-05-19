use std::path::PathBuf;
use std::process::{Command, Output};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_no-mistakes"))
}

fn fixture(category: &str, scenario: &str) -> PathBuf {
    no_mistakes_core::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/rules")
            .join(category)
            .join(scenario),
    )
}

fn check(root: &PathBuf, yaml: &str) -> Output {
    let config = tempfile::Builder::new().suffix(".yml").tempfile().unwrap();
    std::fs::write(config.path(), yaml).unwrap();
    Command::new(bin())
        .args(["check", "--root"])
        .arg(root)
        .arg("--config")
        .arg(config.path())
        .output()
        .unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

// ── agents-md-max-size ────────────────────────────────────────────────────────

#[test]
fn agents_md_max_size_passes_under_limits() {
    let root = fixture("agents-md-max-size", "pass");
    let out = check(
        &root,
        "rules:\n  agents-md-max-size:\n    enabled: true\n    maxLines: 5\n    maxChars: 1000\n",
    );
    assert!(out.status.success(), "exit non-zero: {}", stdout(&out));
    assert!(
        stdout(&out).is_empty() || !stdout(&out).contains("lines"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn agents_md_max_size_fails_over_line_limit() {
    let root = fixture("agents-md-max-size", "fail");
    let out = check(
        &root,
        "rules:\n  agents-md-max-size:\n    enabled: true\n    maxLines: 2\n",
    );
    assert!(!out.status.success(), "expected exit 1 for over-limit file");
    assert!(stdout(&out).contains("3 lines"), "{}", stdout(&out));
    assert!(stdout(&out).contains("CLAUDE.md"), "{}", stdout(&out));
}

#[test]
fn agents_md_max_size_json_output_includes_rule_id() {
    let root = fixture("agents-md-max-size", "fail");
    // Re-run with --format json
    let config = tempfile::Builder::new().suffix(".yml").tempfile().unwrap();
    std::fs::write(
        config.path(),
        "rules:\n  agents-md-max-size:\n    enabled: true\n    maxLines: 2\n",
    )
    .unwrap();
    let out_json = Command::new(bin())
        .args(["check", "--root"])
        .arg(&root)
        .arg("--config")
        .arg(config.path())
        .args(["--format", "json"])
        .output()
        .unwrap();
    let body = stdout(&out_json);
    assert!(
        body.contains("agents-md-max-size"),
        "rule id missing: {body}"
    );
    assert!(!out_json.status.success());
}

#[test]
fn agents_md_max_size_disabled_skips_check() {
    let root = fixture("agents-md-max-size", "fail");
    let out = check(
        &root,
        "rules:\n  agents-md-max-size:\n    enabled: false\n    maxLines: 2\n",
    );
    assert!(
        out.status.success(),
        "disabled rule should not fail: {}",
        stdout(&out)
    );
}

// ── rust-max-lines-per-file ───────────────────────────────────────────────────

#[test]
fn rust_max_lines_per_file_passes_under_limit() {
    let root = fixture("rust-max-lines-per-file", "pass");
    let out = check(
        &root,
        "rules:\n  rust-max-lines-per-file:\n    enabled: true\n    srcMax: 20\n",
    );
    assert!(out.status.success(), "exit non-zero: {}", stdout(&out));
}

#[test]
fn rust_max_lines_per_file_fails_over_limit() {
    let root = fixture("rust-max-lines-per-file", "fail");
    let out = check(
        &root,
        "rules:\n  rust-max-lines-per-file:\n    enabled: true\n    srcMax: 3\n",
    );
    assert!(!out.status.success(), "expected exit 1");
    assert!(stdout(&out).contains("code lines"), "{}", stdout(&out));
    assert!(stdout(&out).contains("big.rs"), "{}", stdout(&out));
}

#[test]
fn rust_max_lines_per_file_disabled_skips() {
    let root = fixture("rust-max-lines-per-file", "fail");
    let out = check(
        &root,
        "rules:\n  rust-max-lines-per-file:\n    enabled: false\n    srcMax: 3\n",
    );
    assert!(
        out.status.success(),
        "disabled rule must not fail: {}",
        stdout(&out)
    );
}

#[test]
fn rust_max_lines_per_file_json_has_rule_id() {
    let root = fixture("rust-max-lines-per-file", "fail");
    let config = tempfile::Builder::new().suffix(".yml").tempfile().unwrap();
    std::fs::write(
        config.path(),
        "rules:\n  rust-max-lines-per-file:\n    enabled: true\n    srcMax: 3\n",
    )
    .unwrap();
    let out = Command::new(bin())
        .args(["check", "--root"])
        .arg(&root)
        .arg("--config")
        .arg(config.path())
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert!(
        stdout(&out).contains("rust-max-lines-per-file"),
        "{}",
        stdout(&out)
    );
}

// ── rust-no-inline-tests ──────────────────────────────────────────────────────

#[test]
fn rust_no_inline_tests_passes_out_of_line() {
    let root = fixture("rust-no-inline-tests", "pass");
    let out = check(
        &root,
        "rules:\n  rust-no-inline-tests:\n    enabled: true\n",
    );
    assert!(out.status.success(), "exit non-zero: {}", stdout(&out));
}

#[test]
fn rust_no_inline_tests_fails_inline_block() {
    let root = fixture("rust-no-inline-tests", "fail");
    let out = check(
        &root,
        "rules:\n  rust-no-inline-tests:\n    enabled: true\n",
    );
    assert!(!out.status.success(), "expected exit 1");
    assert!(stdout(&out).contains("inline"), "{}", stdout(&out));
    assert!(stdout(&out).contains("lib.rs"), "{}", stdout(&out));
}

#[test]
fn rust_no_inline_tests_disabled_skips() {
    let root = fixture("rust-no-inline-tests", "fail");
    let out = check(
        &root,
        "rules:\n  rust-no-inline-tests:\n    enabled: false\n",
    );
    assert!(
        out.status.success(),
        "disabled rule must not fail: {}",
        stdout(&out)
    );
}

#[test]
fn rust_no_inline_tests_json_has_rule_id() {
    let root = fixture("rust-no-inline-tests", "fail");
    let config = tempfile::Builder::new().suffix(".yml").tempfile().unwrap();
    std::fs::write(
        config.path(),
        "rules:\n  rust-no-inline-tests:\n    enabled: true\n",
    )
    .unwrap();
    let out = Command::new(bin())
        .args(["check", "--root"])
        .arg(&root)
        .arg("--config")
        .arg(config.path())
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert!(
        stdout(&out).contains("rust-no-inline-tests"),
        "{}",
        stdout(&out)
    );
    assert!(!out.status.success());
}

// ── gitignored files are skipped ─────────────────────────────────────────────

#[test]
fn agents_md_max_size_skips_gitignored_files() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // Over-limit file in a gitignored directory
    std::fs::create_dir_all(root.join("ignored")).unwrap();
    let big: String = "line\n".repeat(300);
    std::fs::write(root.join("ignored/AGENTS.md"), &big).unwrap();

    // Passing tracked file
    std::fs::write(root.join("CLAUDE.md"), "# ok\n").unwrap();

    // .gitignore excludes the directory
    std::fs::write(root.join(".gitignore"), "ignored/\n").unwrap();

    // Initialise a git repo and commit so git ls-files is the source of truth
    assert!(Command::new("git")
        .args(["-C", root.to_str().unwrap(), "init", "-q"])
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args(["-C", root.to_str().unwrap(), "add", "."])
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args([
            "-C",
            root.to_str().unwrap(),
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-qm",
            "init"
        ])
        .status()
        .unwrap()
        .success());

    let config = tempfile::Builder::new().suffix(".yml").tempfile().unwrap();
    std::fs::write(
        config.path(),
        "rules:\n  agents-md-max-size:\n    enabled: true\n    maxLines: 5\n",
    )
    .unwrap();
    let out = Command::new(bin())
        .args(["check", "--root"])
        .arg(root)
        .arg("--config")
        .arg(config.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "gitignored files must not be flagged: {}",
        stdout(&out)
    );
}

#[test]
fn rust_no_inline_tests_skips_gitignored_files() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("generated")).unwrap();
    std::fs::write(
        root.join("generated/lib.rs"),
        "#[cfg(test)]\nmod tests {\n}\n",
    )
    .unwrap();
    std::fs::write(root.join("clean.rs"), "pub fn ok() {}\n").unwrap();
    std::fs::write(root.join(".gitignore"), "generated/\n").unwrap();

    assert!(Command::new("git")
        .args(["-C", root.to_str().unwrap(), "init", "-q"])
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args(["-C", root.to_str().unwrap(), "add", "."])
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args([
            "-C",
            root.to_str().unwrap(),
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-qm",
            "init"
        ])
        .status()
        .unwrap()
        .success());

    let config = tempfile::Builder::new().suffix(".yml").tempfile().unwrap();
    std::fs::write(
        config.path(),
        "rules:\n  rust-no-inline-tests:\n    enabled: true\n",
    )
    .unwrap();
    let out = Command::new(bin())
        .args(["check", "--root"])
        .arg(root)
        .arg("--config")
        .arg(config.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "gitignored files must not be flagged: {}",
        stdout(&out)
    );
}
