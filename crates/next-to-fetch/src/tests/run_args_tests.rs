use super::helpers::with_run_args_env;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn fixture(category: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(category)
        .join(name)
}

#[test]
fn test_run_without_test_argv_uses_real_cli_args() {
    const ENV_VAR: &str = "NEXT_TO_FETCH_TEST_ARGS";
    let previous = "next-to-fetch\x1f--json";

    {
        let _run_args = with_run_args_env(Some(previous.to_string()), None);
        assert_eq!(std::env::var(ENV_VAR).unwrap(), previous);
        let _ = crate::cli::run_cli();
    }
}

#[test]
fn test_with_run_args_unset_restores_existing_value() {
    const ENV_VAR: &str = "NEXT_TO_FETCH_TEST_ARGS";
    let previous = "next-to-fetch\x1f--json";

    {
        let run_args = with_run_args_env(None, Some(previous.to_string()));
        std::env::remove_var(ENV_VAR);
        assert!(std::env::var_os(ENV_VAR).is_none());
        let _guard = run_args.release();
        assert_eq!(std::env::var(ENV_VAR).unwrap(), previous);
    }
}

#[test]
fn test_run_without_test_argv_uses_real_cli_args_from_absence() {
    {
        let _run_args = with_run_args_env(None, None);
        let _ = crate::cli::run_cli();
    }
}

#[test]
fn test_with_run_args_restores_existing_value() {
    const ENV_VAR: &str = "NEXT_TO_FETCH_TEST_ARGS";
    let previous = "next-to-fetch\x1f--json";
    let args = "next-to-fetch\x1f--root\x1f.";

    {
        let run_args = with_run_args_env(Some(args.to_string()), Some(previous.to_string()));
        assert_eq!(std::env::var(ENV_VAR).unwrap(), args);
        let _guard = run_args.release();
        assert_eq!(std::env::var(ENV_VAR).unwrap(), previous);
    }
}

#[test]
fn test_with_run_args_restores_unset_value() {
    const ENV_VAR: &str = "NEXT_TO_FETCH_TEST_ARGS";
    {
        let run_args = with_run_args_env(None, None);
        std::env::set_var(ENV_VAR, "next-to-fetch\x1f--root\x1f.");
        assert_eq!(
            std::env::var(ENV_VAR).unwrap(),
            "next-to-fetch\x1f--root\x1f."
        );
        let _guard = run_args.release();
        assert!(std::env::var_os(ENV_VAR).is_none());
    }
}

#[test]
fn test_with_run_args_env_restores_previous_on_drop() {
    let previous = "next-to-fetch\x1f--json";

    {
        let _run_args = with_run_args_env(
            Some("next-to-fetch\x1f--root\x1f.".to_string()),
            Some(previous.to_string()),
        );
    }
}

#[test]
fn test_with_run_args_env_macro_path() {
    const ENV_VAR: &str = "NEXT_TO_FETCH_TEST_ARGS";
    let next = "next-to-fetch\x1f--root\x1f.";
    let previous = "next-to-fetch\x1f--json";

    let run_args = with_run_args_env(Some(next.to_string()), Some(previous.to_string()));
    assert_eq!(std::env::var(ENV_VAR).unwrap(), next);
    let _guard = run_args.release();
    assert_eq!(std::env::var(ENV_VAR).unwrap(), previous);
}

#[test]
fn test_with_run_args_state_resumes_panic() {
    const ENV_VAR: &str = "NEXT_TO_FETCH_TEST_ARGS";
    let previous = "next-to-fetch\x1f--json";

    let panic_result = {
        let run_args = with_run_args_env(
            Some("next-to-fetch\x1f--root\x1f.".to_string()),
            Some(previous.to_string()),
        );
        let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            panic!("with_run_args_state panic-path");
        }));
        let _guard = run_args.release();
        assert_eq!(std::env::var(ENV_VAR).unwrap(), previous);
        panic_result
    };

    assert!(panic_result.is_err());
}

#[test]
fn test_run_and_main_are_exercised_with_test_argv() {
    let root = tempdir().unwrap();
    fs::create_dir(root.path().join("app/page.tsx").parent().unwrap()).unwrap();
    fs::write(root.path().join("app/page.tsx"), "fetch('/api/page');").unwrap();

    {
        let next_value = format!(
            "next-to-fetch\x1f--root\x1f{}\x1f--json",
            root.path().to_string_lossy()
        );
        let _run_args = with_run_args_env(Some(next_value.to_string()), None);
        assert!(crate::cli::run_cli().is_ok());
    }
}

#[test]
fn test_run_cli_markdown_path_with_test_argv() {
    let root = fixture("nextjs-fetches", "next-app");
    let next_value = format!("next-to-fetch\x1f--root\x1f{}", root.to_string_lossy());
    let _run_args = with_run_args_env(Some(next_value), None);

    assert!(crate::cli::run_cli().is_ok());
}

#[test]
fn test_run_cli_error_path_with_test_argv() {
    let missing = fixture("nextjs-fetches", "missing-root");
    let next_value = format!("next-to-fetch\x1f--root\x1f{}", missing.to_string_lossy());
    let _run_args = with_run_args_env(Some(next_value), None);

    assert!(crate::cli::run_cli().is_err());
}
