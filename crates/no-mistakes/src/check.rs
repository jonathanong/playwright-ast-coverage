use crate::check_runner;
use anyhow::{Context, Result};
use clap::Args;
use no_mistakes_core::cli::{resolve_root, Format};
use no_mistakes_core::codebase::rules::RuleFinding;
use no_mistakes_core::codebase::unique_exports::UniqueExportFinding;
use no_mistakes_core::integration_tests::IntegrationFinding;
use no_mistakes_core::queue::CheckFinding;
use no_mistakes_core::react_traits;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

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
    /// Print per-check timing information to stderr.
    #[arg(long, global = true)]
    timings: bool,
}

pub(crate) fn run(args: CheckArgs) -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("cwd must be accessible")?;
    let root = resolve_root(&args.root, &cwd);
    let results = check_runner::run_all(root, args.config, args.tsconfig)?;
    for warning in &results.warnings {
        eprintln!("{warning}");
    }

    let any_violations = results.has_findings();
    let format = if args.json { Format::Json } else { args.format };
    match format {
        Format::Json => print_check_json(&results),
        Format::Yml => print_check_yml(&results),
        Format::Md => print_check_md(&results),
        Format::Paths => print_check_paths(&results),
        Format::Human => print_check_human(&results),
    }

    if args.timings {
        print_timings(&results.timings);
    }

    Ok(if any_violations {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn print_timings(timings: &[(&str, Duration)]) {
    for (label, duration) in timings {
        eprintln!("{label}: {:.3}ms", duration.as_secs_f64() * 1000.0);
    }
}

fn print_check_json(results: &check_runner::CheckResults) {
    println!(
        "{}",
        serde_json::to_string_pretty(&json_value(results))
            .expect("serialization of Rust structs never fails")
    );
}

fn print_check_yml(results: &check_runner::CheckResults) {
    println!(
        "{}",
        serde_yaml::to_string(&json_value(results))
            .expect("serialization of Rust structs never fails")
    );
}

fn print_check_md(results: &check_runner::CheckResults) {
    println!("# no-mistakes check");
    println!("## react");
    for v in &results.react {
        println!("- `{}` `{}`: {}", v.file, v.component, v.rule);
    }
    println!("## queues");
    for f in &results.queues {
        println!("- `{}`:{} {}", f.file, f.line, f.message);
    }
    println!("## rules");
    for f in &results.rules {
        println!("- `{}`:{} {} {}", f.file, f.line, f.rule, f.message);
    }
    println!("## integration");
    for f in &results.integration {
        println!("- `{}`:{} {}", f.file, f.line, f.message);
    }
    println!("## codebase");
    for f in &results.codebase {
        println!(
            "- `{}`:{} `{}` {}",
            f.file, f.line, f.export_name, f.message
        );
    }
}

fn print_check_paths(results: &check_runner::CheckResults) {
    for v in &results.react {
        println!("{}", v.file);
    }
    for f in &results.queues {
        println!("{}:{}", f.file, f.line);
    }
    for f in &results.rules {
        println!("{}:{}", f.file, f.line);
    }
    for f in &results.integration {
        println!("{}:{}", f.file, f.line);
    }
    for f in &results.codebase {
        println!("{}:{}:{}", f.file, f.line, f.export_name);
    }
}

fn print_check_human(results: &check_runner::CheckResults) {
    if !results.react.is_empty() {
        react_traits::print_violations(&results.react);
    }
    print_queue_human(&results.queues);
    print_rules_human(&results.rules);
    print_integration_human(&results.integration);
    print_codebase_human(&results.codebase);
}

fn print_queue_human(findings: &[CheckFinding]) {
    for f in findings {
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

fn print_rules_human(findings: &[RuleFinding]) {
    for f in findings {
        println!("{} {}:{} {}", f.rule, f.file, f.line, f.message);
    }
}

fn print_integration_human(findings: &[IntegrationFinding]) {
    for f in findings {
        println!(
            "integration[{}:{}] {}:{} {}",
            f.framework, f.suite, f.file, f.line, f.message
        );
    }
}

fn print_codebase_human(findings: &[UniqueExportFinding]) {
    for f in findings {
        println!(
            "{}[{}] {}:{} {}",
            f.rule, f.export_name, f.file, f.line, f.message
        );
    }
}

fn json_value(results: &check_runner::CheckResults) -> serde_json::Value {
    serde_json::json!({
        "react": results.react,
        "queues": results.queues,
        "rules": results.rules,
        "integration": results.integration,
        "codebase": results.codebase,
    })
}
