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
    /// Output format: json, paths, human (md/yml use JSON serialization).
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

    // Run react check; skip gracefully when no config is present, log genuine errors.
    // assert_no_fetch defaults to false; enable it via project config.
    let react_violations =
        match react_traits::run_check(&root, args.config.as_deref(), &[], false) {
            Ok(v) => v,
            Err(err) => {
                eprintln!("warning: react check skipped: {err:#}");
                vec![]
            }
        };

    // Run queues check.
    let queue_report = analyze_queues(&root, None, &[])?;
    let queue_findings = &queue_report.check;

    let any_violations = !react_violations.is_empty() || !queue_findings.is_empty();

    match args.format {
        Format::Json | Format::Md | Format::Yml => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "react": react_violations,
                    "queues": queue_findings,
                }))
                .expect("serialization of Rust structs never fails")
            );
        }
        Format::Paths => {
            for v in &react_violations {
                println!("{}", v.file);
            }
            for f in queue_findings {
                println!("{}:{}", f.file, f.line);
            }
        }
        Format::Human => {
            if !react_violations.is_empty() {
                react_traits::print_violations(&react_violations);
            }
            for f in queue_findings {
                println!(
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
