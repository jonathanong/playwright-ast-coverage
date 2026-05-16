use std::process::ExitCode;

fn main() -> ExitCode {
    match server_ast_routes::run_cli() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(2)
        }
    }
}
