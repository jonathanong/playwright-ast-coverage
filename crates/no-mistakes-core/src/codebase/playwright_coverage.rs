use anyhow::{bail, Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use rayon::prelude::*;
use regex::Regex;
use serde::Serialize;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use crate::codebase::config::{load_config, Config, RouteOptions};
use crate::codebase::ts_routes::{defs_frontend, matcher};

const DEFAULT_FRONTEND_ROOT: &str = "web/app";
use crate::codebase::dependencies::Format;
use clap::Args;
use is_terminal::IsTerminal;
use std::io;
use std::io::Write;

#[derive(Args, Debug, Clone)]
pub struct CoverageArgs {
    /// Project root directory (default: current working directory).
    #[arg(long, value_name = "PATH")]
    pub root: Option<PathBuf>,

    /// Next.js App Router root. Overrides route-consistency.frontendRoot.
    #[arg(long, value_name = "PATH")]
    pub frontend_root: Option<PathBuf>,

    /// Playwright test file glob. Defaults cover tests/e2e and playwright specs.
    #[arg(long = "test-glob", value_name = "GLOB")]
    pub test_globs: Vec<String>,

    /// Output format: json, md, yml, paths, human.
    /// Defaults to human on TTY, json on non-TTY.
    #[arg(long, value_name = "FORMAT")]
    pub format: Option<Format>,

    /// Shorthand for --format json.
    #[arg(long, default_value_t = false)]
    pub json: bool,

    /// Emit phase timings to stderr.
    #[arg(long, default_value_t = false)]
    pub timings: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    Covered,
    Uncovered,
}

impl ExitStatus {
    pub fn code(self) -> i32 {
        match self {
            Self::Covered => 0,
            Self::Uncovered => 1,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageReport {
    pub summary: CoverageSummary,
    pub routes: Vec<RouteCoverage>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageSummary {
    pub total: usize,
    pub covered: usize,
    pub uncovered: usize,
    pub coverage_percent: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteCoverage {
    pub route: String,
    pub file: String,
    pub covered: bool,
    pub tests: Vec<RouteTestHit>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct RouteTestHit {
    pub file: String,
    pub url: String,
}

#[derive(Debug, Clone)]
struct PlaywrightVisit {
    file: PathBuf,
    url: String,
}

pub fn run(args: CoverageArgs) -> Result<ExitStatus> {
    let mut timings = crate::codebase::timing::PhaseTimings::start();
    let cwd = std::env::current_dir().expect("current directory is readable");
    let root = resolve_root(args.root.as_deref(), &cwd);
    let root = crate::codebase::ts_resolver::normalize_path(&root);
    let config = match load_config(&root) {
        Ok(config) => Some(config),
        Err(err) if args.frontend_root.is_some() => {
            eprintln!(
                "warning: ignoring guardrails config load error because --frontend-root was provided: {err:#}"
            );
            None
        }
        Err(err) => return Err(err).context("loading guardrails config"),
    };
    let frontend_root =
        resolve_frontend_root(args.frontend_root.as_deref(), &root, config.as_ref())?;

    timings.mark("search");

    let extra_skip = config
        .as_ref()
        .map(|config| config.filesystem.skip_directories.as_slice())
        .unwrap_or(&[]);
    let mut all_files = crate::codebase::ts_source::discover_files(&root, extra_skip);
    if let Some(config) = &config {
        all_files =
            filter_skip_file_patterns(&root, all_files, &config.filesystem.skip_file_patterns);
    }
    timings.mark("ingest");

    let report = collect_report_with_frontend_root(
        &root,
        &frontend_root,
        test_globs_or_default(&args.test_globs),
        &all_files,
    )?;
    timings.mark("parse+analysis");

    if report.summary.total == 0 {
        bail!(
            "no Next.js routes discovered under {}",
            frontend_root.display()
        );
    }

    let format = resolve_format(args.json, args.format, io::stdout().is_terminal());

    let stdout = io::stdout();
    let mut out = stdout.lock();
    write_report(&report, format, &mut out)?;

    timings.mark("output");
    if args.timings {
        timings.print_stderr();
    }

    if report.summary.uncovered == 0 {
        Ok(ExitStatus::Covered)
    } else {
        Ok(ExitStatus::Uncovered)
    }
}

fn resolve_format(json: bool, format: Option<Format>, stdout_is_terminal: bool) -> Format {
    if json {
        Format::Json
    } else if let Some(format) = format {
        format
    } else if stdout_is_terminal {
        Format::Human
    } else {
        Format::Json
    }
}

pub(crate) fn collect_report_from_files(
    root: &Path,
    frontend_root: Option<&Path>,
    test_globs: &[String],
    all_files: &[PathBuf],
) -> Result<CoverageReport> {
    let config = load_config(root).context("loading guardrails config")?;
    let frontend_root = resolve_frontend_root(frontend_root, root, Some(&config))?;
    let all_files = filter_skip_file_patterns(
        root,
        all_files.to_vec(),
        &config.filesystem.skip_file_patterns,
    );
    collect_report_with_frontend_root(
        root,
        &frontend_root,
        test_globs_or_default(test_globs),
        &all_files,
    )
}

fn resolve_root(arg: Option<&Path>, cwd: &Path) -> PathBuf {
    match arg {
        Some(path) if path.is_absolute() => path.to_path_buf(),
        Some(path) => cwd.join(path),
        None => cwd.to_path_buf(),
    }
}

fn resolve_frontend_root(
    arg: Option<&Path>,
    root: &Path,
    config: Option<&Config>,
) -> Result<PathBuf> {
    if let Some(path) = arg {
        let frontend_root = if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.join(path)
        };
        return validate_frontend_root(frontend_root);
    }

    let Some(config) = config else {
        return default_frontend_root(root);
    };
    let opts: RouteOptions = config.rule_options("route-consistency");
    if opts == RouteOptions::default() || opts.frontend_root.is_empty() {
        return default_frontend_root(root);
    }

    validate_frontend_root(root.join(opts.frontend_root))
}

fn default_frontend_root(root: &Path) -> Result<PathBuf> {
    let default = root.join(DEFAULT_FRONTEND_ROOT);
    if default.is_dir() {
        return Ok(default);
    }

    bail!(
        "could not determine Next.js App Router root; pass --frontend-root or configure route-consistency.frontendRoot"
    )
}

fn validate_frontend_root(frontend_root: PathBuf) -> Result<PathBuf> {
    if frontend_root.is_dir() {
        Ok(frontend_root)
    } else {
        bail!(
            "Next.js App Router root does not exist: {}",
            frontend_root.display()
        )
    }
}

#[inline(never)]
fn filter_skip_file_patterns(
    root: &Path,
    files: Vec<PathBuf>,
    skip_file_patterns: &[String],
) -> Vec<PathBuf> {
    let mut patterns = Vec::new();
    for pattern in skip_file_patterns {
        if let Ok(pattern) = Regex::new(pattern) {
            patterns.push(pattern);
        }
    }
    if patterns.is_empty() {
        return files;
    }

    let mut filtered = Vec::new();
    for path in files {
        let Ok(rel) = path.strip_prefix(root) else {
            filtered.push(path);
            continue;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        if !matches_any_pattern(&patterns, &rel) {
            filtered.push(path);
        }
    }
    filtered
}

fn matches_any_pattern(patterns: &[Regex], rel: &str) -> bool {
    for pattern in patterns {
        if pattern.is_match(rel) {
            return true;
        }
    }
    false
}

fn test_globs_or_default(globs: &[String]) -> Vec<String> {
    if globs.is_empty() {
        crate::codebase::dependencies::test_globs("playwright")
    } else {
        globs.to_vec()
    }
}

#[inline(never)]
fn collect_report_with_frontend_root(
    root: &Path,
    frontend_root: &Path,
    test_globs: Vec<String>,
    all_files: &[PathBuf],
) -> Result<CoverageReport> {
    let routes = defs_frontend::collect_frontend_routes_from_files(frontend_root, all_files);
    let visits: Vec<(String, String)> = collect_playwright_visits(root, &test_globs, all_files)?
        .into_iter()
        .map(|visit| (visit.url, relative_string(root, &visit.file)))
        .collect();

    let mut route_coverages: Vec<RouteCoverage> = routes
        .into_par_iter()
        .map(|(file, route)| {
            let mut tests: Vec<RouteTestHit> = visits
                .iter()
                .filter(|(url, _)| matcher::matches(url, &route))
                .map(|(url, rel_file)| RouteTestHit {
                    file: rel_file.clone(),
                    url: url.clone(),
                })
                .collect();
            tests.sort();
            RouteCoverage {
                route,
                file: relative_string(root, &file),
                covered: !tests.is_empty(),
                tests,
            }
        })
        .collect();

    route_coverages.sort_by(compare_route_coverage);

    let total = route_coverages.len();
    let covered = route_coverages.iter().filter(|route| route.covered).count();
    let uncovered = total.saturating_sub(covered);
    let coverage_percent = if total == 0 {
        100.0
    } else {
        (covered as f64 / total as f64) * 100.0
    };

    Ok(CoverageReport {
        summary: CoverageSummary {
            total,
            covered,
            uncovered,
            coverage_percent,
        },
        routes: route_coverages,
    })
}

fn compare_route_coverage(a: &RouteCoverage, b: &RouteCoverage) -> Ordering {
    let route_order = a.route.cmp(&b.route);
    if route_order != Ordering::Equal {
        return route_order;
    }
    a.file.cmp(&b.file)
}

fn collect_playwright_visits(
    root: &Path,
    test_globs: &[String],
    all_files: &[PathBuf],
) -> Result<Vec<PlaywrightVisit>> {
    let globset = build_globset(test_globs)?;
    let mut visits: Vec<PlaywrightVisit> = all_files
        .par_iter()
        .filter(|path| {
            path.strip_prefix(root)
                .map(|rel| globset.is_match(rel))
                .unwrap_or(false)
        })
        .flat_map_iter(|path| {
            let Ok(source) = std::fs::read_to_string(path) else {
                return Vec::new();
            };
            crate::codebase::dependencies::graph::playwright::extract_playwright_urls(&source)
                .into_iter()
                .map(|url| PlaywrightVisit {
                    file: path.clone(),
                    url,
                })
                .collect::<Vec<_>>()
        })
        .collect();
    visits.sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.url.cmp(&b.url)));
    visits.dedup_by(|a, b| a.file == b.file && a.url == b.url);
    Ok(visits)
}

fn build_globset(globs: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for glob in globs {
        builder.add(Glob::new(glob).context(format!("invalid glob `{glob}`"))?);
    }
    Ok(builder.build()?)
}

fn relative_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

fn write_report(report: &CoverageReport, format: Format, out: &mut dyn Write) -> Result<()> {
    match format {
        Format::Json => write_json(report, out),
        Format::Md => write_markdown(report, out),
        Format::Yml => write_yml(report, out),
        Format::Paths => write_paths(report, out),
        Format::Human => write_human(report, out),
    }
}

fn write_json(report: &CoverageReport, out: &mut dyn Write) -> Result<()> {
    serde_json::to_writer_pretty(&mut *out, report)
        .context("serializing coverage report to JSON")?;
    writeln!(out)?;
    Ok(())
}

fn write_yml(report: &CoverageReport, out: &mut dyn Write) -> Result<()> {
    let yml = serde_yaml::to_string(report).context("serializing coverage report to YAML")?;
    out.write_all(yml.as_bytes())?;
    Ok(())
}

fn write_paths(report: &CoverageReport, out: &mut dyn Write) -> Result<()> {
    for route in report.routes.iter().filter(|route| !route.covered) {
        writeln!(out, "{}", route.file)?;
    }
    Ok(())
}

fn write_human(report: &CoverageReport, out: &mut dyn Write) -> Result<()> {
    let line = format!(
        "Playwright route coverage: {}/{} ({:.1}%)",
        report.summary.covered, report.summary.total, report.summary.coverage_percent
    );
    writeln!(out, "{line}")?;

    if report.summary.uncovered == 0 {
        writeln!(out, "All routes are covered.")?;
        return Ok(());
    }

    writeln!(out, "Uncovered routes:")?;
    for route in report.routes.iter().filter(|route| !route.covered) {
        writeln!(out, "  {} ({})", route.route, route.file)?;
    }
    Ok(())
}

fn write_markdown(report: &CoverageReport, out: &mut dyn Write) -> Result<()> {
    let header = format!(
        "# Playwright route coverage\n\n- Covered: {}/{}\n- Coverage: {:.1}%\n",
        report.summary.covered, report.summary.total, report.summary.coverage_percent
    );
    writeln!(out, "{header}")?;

    if report.summary.uncovered == 0 {
        writeln!(out, "_All routes are covered._")?;
        return Ok(());
    }

    writeln!(out, "## Uncovered routes\n")?;
    for route in report.routes.iter().filter(|route| !route.covered) {
        writeln!(out, "- `{}` ({})", route.route, route.file)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
