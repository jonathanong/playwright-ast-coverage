use anyhow::{Context, Result};
use clap::Args;
use no_mistakes_core::cli::{resolve_root, Format};
use no_mistakes_core::codebase::rules::{self, RuleFinding};
use no_mistakes_core::queue::{analyze_project as analyze_queues, CheckFinding};
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
    /// Path to tsconfig.json for queue import alias resolution.
    #[arg(long, global = true)]
    tsconfig: Option<PathBuf>,
    /// Output format: json, yml, md, paths, human.
    #[arg(
        long,
        value_enum,
        default_value = "human",
        global = true,
        conflicts_with = "json"
    )]
    format: Format,
    /// Shorthand for --format json.
    #[arg(long, global = true, conflicts_with = "format")]
    json: bool,
}

pub(crate) fn run(args: CheckArgs) -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("cwd must be accessible")?;
    let root = resolve_root(&args.root, &cwd);

    // Run react check; skip gracefully when no config is present, log genuine errors.
    // assert_no_fetch defaults to false; enable it via project config.
    let react_violations = match react_traits::run_check(&root, args.config.as_deref(), &[], false)
    {
        Ok(v) => v,
        Err(err) => {
            eprintln!("warning: react check skipped: {err:#}");
            vec![]
        }
    };

    // Run queues check.
    let queue_report = analyze_queues(&root, args.tsconfig.as_deref(), &[])?;
    let queue_findings = &queue_report.check;
    let rule_findings =
        match rules::run_check(&root, args.config.as_deref(), args.tsconfig.as_deref()) {
            Ok(findings) => findings,
            Err(err) => {
                eprintln!("warning: rules check skipped: {err:#}");
                vec![]
            }
        };

    let any_violations =
        !react_violations.is_empty() || !queue_findings.is_empty() || !rule_findings.is_empty();

    let format = if args.json { Format::Json } else { args.format };
    match format {
        Format::Json => println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "react": react_violations,
                "queues": queue_findings,
                "rules": rule_findings,
            }))
            .expect("serialization of Rust structs never fails")
        ),
        Format::Yml => println!(
            "{}",
            serde_yaml::to_string(&serde_json::json!({
                "react": react_violations,
                "queues": queue_findings,
                "rules": rule_findings,
            }))
            .expect("serialization of Rust structs never fails")
        ),
        Format::Md => print_check_md(&react_violations, queue_findings, &rule_findings),
        Format::Paths => {
            for v in &react_violations {
                println!("{}", v.file);
            }
            for f in queue_findings {
                println!("{}:{}", f.file, f.line);
            }
            for f in &rule_findings {
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
            for f in &rule_findings {
                println!("{} {}:{} {}", f.rule, f.file, f.line, f.message);
            }
        }
    }

    Ok(if any_violations {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn print_check_md(
    react: &[react_traits::Violation],
    queues: &[CheckFinding],
    rules: &[RuleFinding],
) {
    println!("# no-mistakes check");
    println!("## react");
    for v in react {
        println!("- `{}` `{}`: {}", v.file, v.component, v.rule);
    }
    println!("## queues");
    for f in queues {
        println!("- `{}`:{} {}", f.file, f.line, f.message);
    }
    println!("## rules");
    for f in rules {
        println!("- `{}`:{} {} {}", f.file, f.line, f.rule, f.message);
    }
}
