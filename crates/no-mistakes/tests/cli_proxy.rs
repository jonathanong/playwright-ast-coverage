use std::env;
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_no-mistakes"))
}

fn proxy_fixture(name: &str) -> PathBuf {
    no_mistakes_core::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/no-mistakes-proxy")
            .join(name),
    )
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be utf8")
}

fn proxy_path_with(paths: &[PathBuf]) -> std::ffi::OsString {
    env::join_paths(
        paths
            .iter()
            .cloned()
            .chain(env::split_paths(&env::var_os("PATH").unwrap_or_default())),
    )
    .expect("PATH should join")
}

fn proxy_path() -> std::ffi::OsString {
    proxy_path_with(&[proxy_fixture("bin")])
}

#[test]
fn external_subcommand_proxies_to_no_mistakes_executable_on_path() {
    let output = Command::new(bin())
        .env("PATH", proxy_path())
        .args(["fixture-proxy", "--print", "one"])
        .output()
        .expect("no-mistakes should run");

    assert!(output.status.success());
    assert_eq!(stdout(&output).trim(), "fixture-proxy:--print:one");
}

#[test]
fn external_subcommand_preserves_proxy_exit_status() {
    let output = Command::new(bin())
        .env("PATH", proxy_path())
        .args(["fixture-proxy", "--fail"])
        .output()
        .expect("no-mistakes should run");

    assert_eq!(output.status.code(), Some(7));
    assert!(stdout(&output).contains("proxy failed"));
}

#[cfg(unix)]
#[test]
fn external_subcommand_maps_signal_exit_status() {
    let output = Command::new(bin())
        .env("PATH", proxy_path())
        .args(["fixture-proxy", "--signal-int"])
        .output()
        .expect("no-mistakes should run");

    assert_eq!(output.status.code(), Some(130));
}

#[test]
fn external_subcommand_reports_missing_executable() {
    let output = Command::new(bin())
        .env("PATH", "")
        .arg("missing-proxy")
        .output()
        .expect("no-mistakes should run");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no-mistakes-missing-proxy"));
}

#[cfg(unix)]
#[test]
fn external_subcommand_skips_non_executable_path_match() {
    let output = Command::new(bin())
        .env(
            "PATH",
            proxy_path_with(&[proxy_fixture("non-executable-bin"), proxy_fixture("bin")]),
        )
        .args(["fixture-proxy", "--print", "fallback"])
        .output()
        .expect("no-mistakes should run");

    assert!(output.status.success());
    assert_eq!(stdout(&output).trim(), "fixture-proxy:--print:fallback");
}

#[cfg(unix)]
#[test]
fn external_subcommand_skips_file_current_user_cannot_execute() {
    let blocked = proxy_fixture("owner-blocked-bin/no-mistakes-fixture-proxy");
    let original_permissions = fs::metadata(&blocked)
        .expect("blocked fixture should exist")
        .permissions();

    let mut blocked_permissions = original_permissions.clone();
    blocked_permissions.set_mode(0o001);
    fs::set_permissions(&blocked, blocked_permissions).expect("fixture mode should change");

    let output = Command::new(bin())
        .env(
            "PATH",
            proxy_path_with(&[proxy_fixture("owner-blocked-bin"), proxy_fixture("bin")]),
        )
        .args(["fixture-proxy", "--print", "fallback"])
        .output()
        .expect("no-mistakes should run");

    fs::set_permissions(&blocked, original_permissions).expect("fixture mode should restore");

    assert!(output.status.success());
    assert_eq!(stdout(&output).trim(), "fixture-proxy:--print:fallback");
}
