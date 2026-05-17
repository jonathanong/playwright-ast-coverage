use anyhow::Result;
use clap::Args;
use no_mistakes_core::cli::Format;
use no_mistakes_core::queue::analyze_project as analyze_queues;
use no_mistakes_core::react_traits;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Args, Debug)]
pub(crate) struct CheckArgs {
    /// Project root directory.
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,
    /// Path to config file.
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    /// Output format: json, paths, human.
    #[arg(long, value_enum, default_value = "human", global = true)]
    format: Format,
}

pub(crate) fn run(args: CheckArgs) -> Result<ExitCode> {
    let cwd = std::env::current_dir()?;
    let root = if args.root.is_absolute() {
        args.root.clone()
    } else {
        cwd.join(&args.root)
    };

    let mut any_violations = false;

    // Run react check; if no frontend root exists, skip gracefully.
    match react_traits::run_check(&root, args.config.as_deref(), &[], false) {
        Ok(react_violations) if !react_violations.is_empty() => {
            any_violations = true;
            if matches!(args.format, Format::Json | Format::Md | Format::Yml) {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&react_violations)
                        .expect("serialization of Rust structs never fails")
                );
            } else {
                react_traits::print_violations(&react_violations);
            }
        }
        Ok(_) => {}
        Err(_) => {
            // React analysis is optional; skip when not applicable.
        }
    }

    // Run queues check.
    let queue_report = analyze_queues(&root, None, &[])?;
    if !queue_report.check.is_empty() {
        any_violations = true;
        if matches!(args.format, Format::Json | Format::Md | Format::Yml) {
            println!(
                "{}",
                serde_json::to_string_pretty(&queue_report.check)
                    .expect("serialization of Rust structs never fails")
            );
        } else {
            for f in &queue_report.check {
                eprintln!(
                    "{}[{}] {}:{} {}",
                    f.kind,
                    f.job.as_deref().unwrap_or("*"),
                    f.file,
                    f.line,
                    f.message
                );
            }
        }
    }

    Ok(if any_violations {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}
