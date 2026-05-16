use std::process::ExitCode;

fn main() -> ExitCode {
    match queue_ast_hop::run_cli() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(2)
        }
    }
}
