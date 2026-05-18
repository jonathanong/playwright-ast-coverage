// Integration test helpers and in-process CLI tests

pub(crate) static RUN_ARGS_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) struct RunArgsEnvGuard {
    pub(crate) _guard: Option<std::sync::MutexGuard<'static, ()>>,
    pub(crate) previous: Option<std::ffi::OsString>,
}

impl Drop for RunArgsEnvGuard {
    fn drop(&mut self) {
        const ENV_VAR: &str = "REACT_TRAITS_TEST_ARGS";
        match self.previous.clone() {
            Some(previous) => std::env::set_var(ENV_VAR, previous),
            None => std::env::remove_var(ENV_VAR),
        }
    }
}

impl RunArgsEnvGuard {
    #[allow(dead_code)]
    pub(crate) fn release(mut self) -> std::sync::MutexGuard<'static, ()> {
        const ENV_VAR: &str = "REACT_TRAITS_TEST_ARGS";
        match self.previous.take() {
            Some(previous) => std::env::set_var(ENV_VAR, previous),
            None => std::env::remove_var(ENV_VAR),
        }
        let guard = self._guard.take().unwrap();
        std::mem::forget(self);
        guard
    }
}

pub(crate) fn with_run_args_env(
    next_value: Option<String>,
    existing: Option<String>,
) -> RunArgsEnvGuard {
    let _guard = RUN_ARGS_MUTEX.lock().unwrap_or_else(|err| err.into_inner());
    const ENV_VAR: &str = "REACT_TRAITS_TEST_ARGS";
    let previous: Option<std::ffi::OsString> = match existing {
        Some(existing) => {
            std::env::set_var(ENV_VAR, &existing);
            Some(existing.into())
        }
        None => {
            std::env::remove_var(ENV_VAR);
            None
        }
    };
    match next_value {
        Some(next_value) => std::env::set_var(ENV_VAR, &next_value),
        None => std::env::remove_var(ENV_VAR),
    }
    RunArgsEnvGuard {
        _guard: Some(_guard),
        previous,
    }
}

fn fixture(category: &str, name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(category)
        .join(name)
}

#[test]
fn cli_analyze_json_output() {
    let root = fixture("react-traits-components", "basic");
    let args = format!(
        "react-traits\x1f--root\x1f{}\x1f--json\x1fanalyze\x1fapp/components/Greeting.tsx",
        root.display()
    );
    let _guard = with_run_args_env(Some(args), None);
    let result = crate::run_cli();
    assert!(result.is_ok());
}

#[test]
fn cli_check_json_output_with_violation() {
    let root = fixture("react-traits-config", "assert-no-fetch");
    let args = format!(
        "react-traits\x1f--root\x1f{}\x1f--json\x1fcheck\x1fapp/components/Fetcher.tsx",
        root.display()
    );
    let _guard = with_run_args_env(Some(args), None);
    let result = crate::run_cli();
    assert!(result.is_ok());
}

#[test]
fn cli_error_exit_bad_root() {
    let args = "react-traits\x1f--root\x1f/nonexistent/path/does/not/exist\x1fanalyze\x1f**/*.tsx"
        .to_string();
    let _guard = with_run_args_env(Some(args), None);
    let result = crate::run_cli();
    // missing frontend root is now an error
    assert!(result.is_err());
}

#[test]
fn cli_analyze_text_output() {
    let root = fixture("react-traits-components", "basic");
    let args = format!(
        "react-traits\x1f--root\x1f{}\x1fanalyze\x1fapp/components/Greeting.tsx",
        root.display()
    );
    let _guard = with_run_args_env(Some(args), None);
    let result = crate::run_cli();
    assert!(result.is_ok());
}

#[test]
fn cli_check_text_output_no_violation() {
    let root = fixture("react-traits-components", "basic");
    let args = format!(
        "react-traits\x1f--root\x1f{}\x1fcheck\x1fapp/components/Greeting.tsx",
        root.display()
    );
    let _guard = with_run_args_env(Some(args), None);
    let result = crate::run_cli();
    assert!(result.is_ok());
}

#[test]
fn cli_check_text_output_with_violation() {
    let root = fixture("react-traits-config", "assert-no-fetch");
    let args = format!(
        "react-traits\x1f--root\x1f{}\x1fcheck\x1f--assert-no-fetch\x1fapp/components/Fetcher.tsx",
        root.display()
    );
    let _guard = with_run_args_env(Some(args), None);
    // Should succeed (returns ExitCode::from(1) but not Err)
    let result = crate::run_cli();
    assert!(result.is_ok());
}

#[test]
fn cli_analyze_bad_file_returns_error() {
    // Exercises the `run_analyze(...)?` error branch in cli.rs.
    let root = fixture("react-traits-components", "bad-file");
    let args = format!(
        "react-traits\x1f--root\x1f{}\x1fanalyze\x1fapp/components/Broken.tsx",
        root.display()
    );
    let _guard = with_run_args_env(Some(args), None);
    let result = crate::run_cli();
    assert!(result.is_err(), "bad file should produce an error");
}

#[test]
fn cli_check_bad_file_returns_error() {
    // Exercises the `run_check(...)?` error branch in cli.rs.
    let root = fixture("react-traits-components", "bad-file");
    let args = format!(
        "react-traits\x1f--root\x1f{}\x1fcheck\x1f--assert-no-fetch\x1fapp/components/Broken.tsx",
        root.display()
    );
    let _guard = with_run_args_env(Some(args), None);
    let result = crate::run_cli();
    assert!(result.is_err(), "bad file should produce an error");
}
