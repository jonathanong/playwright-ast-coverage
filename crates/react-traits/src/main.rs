use std::process::ExitCode;

fn main() -> ExitCode {
    match react_traits::run_cli() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e:#}");
            ExitCode::from(2)
        }
    }
}
