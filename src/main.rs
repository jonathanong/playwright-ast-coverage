mod config;
mod matcher;
mod playwright_config;
mod playwright_urls;
mod routes;
mod selectors;

#[cfg(not(test))]
use anyhow::Context;
use anyhow::Result;
use clap::{Parser, ValueEnum};
use config::Settings;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use rayon::prelude::*;
use routes::Route;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
#[cfg(not(test))]
use std::process::ExitCode;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[arg(long, default_value = ".")]
    root: PathBuf,

    #[arg(long)]
    config: Option<PathBuf>,

    #[arg(long)]
    playwright_config: Option<PathBuf>,

    #[arg(long, value_enum, default_value_t = Mode::Coverage)]
    mode: Mode,

    #[arg(long)]
    json: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum Mode {
    Coverage,
    Edges,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Summary {
    total_routes: usize,
    covered_routes: usize,
    uncovered_routes: usize,
    total_selectors: usize,
    covered_selectors: usize,
    uncovered_selectors: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CoverageRoute {
    route: String,
    file: String,
    covered: bool,
    tests: Vec<String>,
    urls: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CoverageSelector {
    attribute: String,
    value: String,
    file: String,
    covered: bool,
    unsupported_dynamic: bool,
    tests: Vec<String>,
    selectors: Vec<String>,
}

#[derive(Serialize)]
struct CoverageReport {
    summary: Summary,
    routes: Vec<CoverageRoute>,
    selectors: Vec<CoverageSelector>,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Edge {
    #[serde(rename_all = "camelCase")]
    Route {
        test_file: String,
        route_file: String,
        route: String,
        url: String,
    },
    #[serde(rename_all = "camelCase")]
    Selector {
        test_file: String,
        app_file: String,
        attribute: String,
        value: String,
        selector: String,
    },
}

#[derive(Serialize)]
struct EdgeReport {
    edges: Vec<Edge>,
}

struct Analysis {
    coverage: CoverageReport,
    edges: EdgeReport,
}

struct RouteTarget {
    route_file: String,
    pattern: String,
}

struct AppSelectorTarget<'a> {
    selector: &'a selectors::AppSelector,
    app_file: String,
    value: String,
}

struct TestAnalysisContext<'a> {
    root: &'a Path,
    route_targets: &'a [RouteTarget],
    app_selector_targets: &'a [AppSelectorTarget<'a>],
    navigation_helpers: &'a [String],
    base_urls: &'a [String],
    test_id_attributes: &'a [String],
    selector_regexes: &'a selectors::SelectorRegexes,
}

type SelectorCoverageKey = (String, String, String);
type CoverageLinks = (BTreeSet<String>, BTreeSet<String>);

#[cfg(not(test))]
fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(2)
        }
    }
}

#[cfg(not(test))]
fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let root = absolutize(&cli.root).context("failed to resolve --root")?;
    let settings = config::load_settings(
        &root,
        cli.config.as_deref(),
        cli.playwright_config.as_deref(),
    )?;
    let analysis = analyze(&root, &settings)?;

    match cli.mode {
        Mode::Coverage => {
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&analysis.coverage)?);
            } else {
                print_coverage_text(&analysis.coverage);
            }
            if analysis.coverage.summary.uncovered_routes > 0
                || analysis.coverage.summary.uncovered_selectors > 0
            {
                Ok(ExitCode::from(1))
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        Mode::Edges => {
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&analysis.edges)?);
            } else {
                print_edges_text(&analysis.edges);
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn analyze(root: &Path, settings: &Settings) -> Result<Analysis> {
    let route_root = root.join(&settings.frontend_root);
    let routes = routes::collect_routes(&route_root)?;
    if routes.is_empty() {
        anyhow::bail!(
            "no Next.js page routes found under {}",
            route_root
                .strip_prefix(root)
                .unwrap_or(&route_root)
                .display()
        );
    }

    let playwright = playwright_config::load(root, settings.playwright_config.as_deref())?;
    let test_files = discover_test_files(root, settings, &playwright)?;
    let base_urls = playwright.base_urls();
    let test_id_attributes = playwright.test_id_attributes();
    let selector_regexes = selectors::compile_selector_regexes(&settings.selector_attributes);
    let app_selectors = if settings.selector_attributes.is_empty() {
        Vec::new()
    } else {
        collect_app_selectors(root, settings, &selector_regexes)?
    };
    let route_targets = route_targets(root, &routes);
    let app_selector_targets = app_selector_targets(root, &app_selectors);
    let test_analysis = TestAnalysisContext {
        root,
        route_targets: &route_targets,
        app_selector_targets: &app_selector_targets,
        navigation_helpers: &settings.navigation_helpers,
        base_urls: &base_urls,
        test_id_attributes: &test_id_attributes,
        selector_regexes: &selector_regexes,
    };

    let edges: BTreeSet<Edge> = test_files
        .par_iter()
        .try_fold(BTreeSet::new, |mut edges, test_file| -> Result<_> {
            edges.extend(analyze_test_file(test_file, &test_analysis)?);
            Ok(edges)
        })
        .try_reduce(BTreeSet::new, |mut left, right| -> Result<_> {
            left.extend(right);
            Ok(left)
        })?;

    let edge_report = EdgeReport {
        edges: edges.into_iter().collect(),
    };
    let coverage = build_coverage(root, &routes, &app_selectors, &edge_report.edges, settings);
    Ok(Analysis {
        coverage,
        edges: edge_report,
    })
}

fn analyze_test_file(test_file: &Path, context: &TestAnalysisContext<'_>) -> Result<Vec<Edge>> {
    let source = std::fs::read_to_string(test_file)?;
    let rel_test_file = relative_string(context.root, test_file);
    let mut edges = Vec::new();

    for raw_url in playwright_urls::extract_playwright_url_literals_with_helpers(
        &source,
        context.navigation_helpers,
    ) {
        let Some(url) = normalize_url(&raw_url, context.base_urls) else {
            continue;
        };
        for route in context.route_targets {
            if matcher::matches(&url, &route.pattern) {
                edges.push(Edge::Route {
                    test_file: rel_test_file.clone(),
                    route_file: route.route_file.clone(),
                    route: route.pattern.clone(),
                    url: url.clone(),
                });
            }
        }
    }

    if !context.app_selector_targets.is_empty() {
        let playwright_selectors = selectors::extract_playwright_selectors_with_regexes(
            &source,
            context.selector_regexes,
            context.test_id_attributes,
        );
        for app_selector in context.app_selector_targets {
            for playwright_selector in &playwright_selectors {
                if app_selector
                    .selector
                    .matches_playwright(playwright_selector)
                {
                    edges.push(Edge::Selector {
                        test_file: rel_test_file.clone(),
                        app_file: app_selector.app_file.clone(),
                        attribute: app_selector.selector.attribute.clone(),
                        value: app_selector.value.clone(),
                        selector: playwright_selector.selector.clone(),
                    });
                }
            }
        }
    }

    Ok(edges)
}

fn route_targets(root: &Path, routes: &[Route]) -> Vec<RouteTarget> {
    routes
        .iter()
        .map(|route| RouteTarget {
            route_file: relative_string(root, &route.file),
            pattern: route.pattern.clone(),
        })
        .collect()
}

fn app_selector_targets<'a>(
    root: &Path,
    app_selectors: &'a [selectors::AppSelector],
) -> Vec<AppSelectorTarget<'a>> {
    app_selectors
        .iter()
        .map(|selector| AppSelectorTarget {
            selector,
            app_file: relative_string(root, &selector.file),
            value: selector.display_value(),
        })
        .collect()
}

fn collect_app_selectors(
    root: &Path,
    settings: &Settings,
    selector_regexes: &selectors::SelectorRegexes,
) -> Result<Vec<selectors::AppSelector>> {
    let include = build_globset(&settings.selector_include)?;
    let exclude = build_globset(&settings.selector_exclude)?;
    let include_all = settings.selector_include.is_empty();
    let source_files =
        collect_selector_source_files(root, settings, &include, &exclude, include_all);
    let app_selectors = source_files
        .par_iter()
        .try_fold(BTreeSet::new, |mut app_selectors, path| -> Result<_> {
            let source = std::fs::read_to_string(path)?;
            app_selectors.extend(selectors::extract_app_selectors_with_regexes(
                path,
                &source,
                selector_regexes,
            ));
            Ok(app_selectors)
        })
        .try_reduce(BTreeSet::new, |mut left, right| -> Result<_> {
            left.extend(right);
            Ok(left)
        })?;

    Ok(app_selectors.into_iter().collect())
}

fn collect_selector_source_files(
    root: &Path,
    settings: &Settings,
    include: &GlobSet,
    exclude: &GlobSet,
    include_all: bool,
) -> Vec<PathBuf> {
    let mut source_files = BTreeSet::new();
    for selector_root in &settings.selector_roots {
        let source_root = root.join(selector_root);
        if !source_root.exists() {
            continue;
        }

        for path in walk_files(&source_root) {
            if !selectors::is_source_file(&path) {
                continue;
            }
            let rel = relative_string(root, &path);
            if (!include_all && !include.is_match(&rel)) || exclude.is_match(&rel) {
                continue;
            }

            source_files.insert(path);
        }
    }

    source_files.into_iter().collect()
}

fn discover_test_files(
    root: &Path,
    settings: &Settings,
    playwright: &playwright_config::PlaywrightConfig,
) -> Result<Vec<PathBuf>> {
    if !settings.test_include.is_empty() {
        let include = build_globset(&settings.test_include)?;
        let exclude = build_globset(&settings.test_exclude)?;
        return Ok(walk_files(root)
            .into_iter()
            .filter(|path| {
                let rel = relative_string(root, path);
                include.is_match(&rel) && !exclude.is_match(&rel)
            })
            .collect());
    }

    let yaml_exclude = build_globset(&settings.test_exclude)?;
    let mut files = BTreeSet::new();

    for project in &playwright.projects {
        let test_dir = project.test_dir(root);
        if !test_dir.exists() {
            continue;
        }
        let include = build_globset(&project.test_match)?;
        let ignore = build_globset(&project.test_ignore)?;

        for path in walk_files(&test_dir) {
            let rel_root = relative_string(root, &path);
            let rel_test = relative_string(&test_dir, &path);
            let abs = slash_path(&path);
            let included = include.is_match(&rel_root)
                || include.is_match(&rel_test)
                || include.is_match(&abs);
            let ignored =
                ignore.is_match(&rel_root) || ignore.is_match(&rel_test) || ignore.is_match(&abs);
            if included && !ignored && !yaml_exclude.is_match(&rel_root) {
                files.insert(path);
            }
        }
    }

    Ok(files.into_iter().collect())
}

fn build_coverage(
    root: &Path,
    routes: &[Route],
    app_selectors: &[selectors::AppSelector],
    edges: &[Edge],
    settings: &Settings,
) -> CoverageReport {
    let ignored: Vec<String> = settings.ignore_routes.clone();
    let mut by_route: BTreeMap<&str, (BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();
    let mut by_selector: BTreeMap<SelectorCoverageKey, CoverageLinks> = BTreeMap::new();

    for edge in edges {
        match edge {
            Edge::Route {
                test_file,
                route,
                url,
                ..
            } => {
                let entry = by_route
                    .entry(route.as_str())
                    .or_insert_with(|| (BTreeSet::new(), BTreeSet::new()));
                entry.0.insert(test_file.clone());
                entry.1.insert(url.clone());
            }
            Edge::Selector {
                test_file,
                app_file,
                attribute,
                value,
                selector,
            } => {
                let entry = by_selector
                    .entry((app_file.clone(), attribute.clone(), value.clone()))
                    .or_insert_with(|| (BTreeSet::new(), BTreeSet::new()));
                entry.0.insert(test_file.clone());
                entry.1.insert(selector.clone());
            }
        }
    }

    let mut coverage_routes = Vec::new();
    for route in routes {
        let (tests, urls) = by_route
            .get(route.pattern.as_str())
            .cloned()
            .unwrap_or_default();
        let covered = !tests.is_empty() || is_ignored(&route.pattern, &ignored);
        coverage_routes.push(CoverageRoute {
            route: route.pattern.clone(),
            file: relative_string(root, &route.file),
            covered,
            tests: tests.into_iter().collect(),
            urls: urls.into_iter().collect(),
        });
    }

    coverage_routes.sort_by(|a, b| a.route.cmp(&b.route).then_with(|| a.file.cmp(&b.file)));
    let mut coverage_selectors = Vec::new();
    for app_selector in app_selectors {
        let app_file = relative_string(root, &app_selector.file);
        let value = app_selector.display_value();
        let (tests, selectors) = by_selector
            .get(&(
                app_file.clone(),
                app_selector.attribute.clone(),
                value.clone(),
            ))
            .cloned()
            .unwrap_or_default();
        let covered = !tests.is_empty();
        coverage_selectors.push(CoverageSelector {
            attribute: app_selector.attribute.clone(),
            value,
            file: app_file,
            covered,
            unsupported_dynamic: app_selector.unsupported_dynamic(),
            tests: tests.into_iter().collect(),
            selectors: selectors.into_iter().collect(),
        });
    }
    coverage_selectors.sort_by(|a, b| {
        a.attribute
            .cmp(&b.attribute)
            .then_with(|| a.value.cmp(&b.value))
            .then_with(|| a.file.cmp(&b.file))
    });

    let total_routes = coverage_routes.len();
    let covered_routes = coverage_routes.iter().filter(|route| route.covered).count();
    let uncovered_routes = total_routes.saturating_sub(covered_routes);
    let total_selectors = coverage_selectors.len();
    let covered_selectors = coverage_selectors
        .iter()
        .filter(|selector| selector.covered)
        .count();
    let uncovered_selectors = total_selectors.saturating_sub(covered_selectors);

    CoverageReport {
        summary: Summary {
            total_routes,
            covered_routes,
            uncovered_routes,
            total_selectors,
            covered_selectors,
            uncovered_selectors,
        },
        routes: coverage_routes,
        selectors: coverage_selectors,
    }
}

fn print_coverage_text(report: &CoverageReport) {
    println!("Routes: {}", report.summary.total_routes);
    println!("Covered routes: {}", report.summary.covered_routes);
    println!("Uncovered routes: {}", report.summary.uncovered_routes);
    println!("Selectors: {}", report.summary.total_selectors);
    println!("Covered selectors: {}", report.summary.covered_selectors);
    println!(
        "Uncovered selectors: {}",
        report.summary.uncovered_selectors
    );

    if report.summary.uncovered_routes == 0 && report.summary.uncovered_selectors == 0 {
        println!();
        println!("All routes and selectors covered.");
        return;
    }

    if report.summary.uncovered_routes > 0 {
        println!();
        println!("Uncovered routes:");
        for route in report.routes.iter().filter(|route| !route.covered) {
            println!("  {}  {}", route.route, route.file);
        }
    }

    if report.summary.uncovered_selectors > 0 {
        println!();
        println!("Uncovered selectors:");
        for selector in report.selectors.iter().filter(|selector| !selector.covered) {
            println!(
                "  [{}=\"{}\"]  {}",
                selector.attribute, selector.value, selector.file
            );
        }
    }
}

fn print_edges_text(report: &EdgeReport) {
    for edge in &report.edges {
        match edge {
            Edge::Route {
                test_file,
                route_file,
                route,
                url,
            } => println!("{test_file} -> {route_file} ({route}, {url})"),
            Edge::Selector {
                test_file,
                app_file,
                attribute,
                value,
                selector,
            } => println!("{test_file} -> {app_file} ([{attribute}=\"{value}\"], {selector})"),
        }
    }
}

fn normalize_url(raw: &str, base_urls: &[String]) -> Option<String> {
    if raw.starts_with('/') {
        return Some(raw.to_string());
    }

    for base_url in base_urls {
        let base = base_url.trim_end_matches('/');
        if let Some(rest) = raw.strip_prefix(base) {
            if rest.is_empty() {
                return Some("/".to_string());
            }
            if rest.starts_with('/') {
                return Some(rest.to_string());
            }
        }
    }

    None
}

fn is_ignored(route: &str, ignored: &[String]) -> bool {
    ignored
        .iter()
        .any(|pattern| route == pattern || matcher::matches(route, pattern))
}

fn build_globset(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = GlobBuilder::new(pattern).literal_separator(false).build()?;
        builder.add(glob);
    }
    Ok(builder.build()?)
}

fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !is_skipped_dir(entry.path()))
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .collect();
    files.sort();
    files
}

fn is_skipped_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name,
                ".git" | "node_modules" | "target" | "dist" | "build" | "coverage" | "test-results"
            )
        })
}

fn relative_string(root: &Path, path: &Path) -> String {
    slash_path(path.strip_prefix(root).unwrap_or(path))
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn absolutize(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_url_handles_relative_absolute_base_and_external() {
        let bases = vec!["http://localhost:3000/".to_string()];
        assert_eq!(
            normalize_url("/users/42", &bases),
            Some("/users/42".to_string())
        );
        assert_eq!(
            normalize_url("http://localhost:3000/users/42", &bases),
            Some("/users/42".to_string())
        );
        assert_eq!(
            normalize_url("http://localhost:3000", &bases),
            Some("/".to_string())
        );
        assert_eq!(normalize_url("http://localhost:3000x", &bases), None);
        assert_eq!(normalize_url("https://example.com/users/42", &bases), None);
    }

    #[test]
    fn ignore_routes_match_exact_and_dynamic_patterns() {
        assert!(is_ignored("/settings", &["/settings".to_string()]));
        assert!(is_ignored("/users/42", &["/users/:id".to_string()]));
        assert!(!is_ignored("/admin", &["/settings".to_string()]));
    }

    #[test]
    fn skipped_directories_are_detected() {
        assert!(is_skipped_dir(Path::new("node_modules")));
        assert!(!is_skipped_dir(Path::new("src")));
    }

    #[test]
    fn path_helpers_handle_absolute_and_relative_paths() {
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(
            absolutize(Path::new("/tmp")).unwrap(),
            PathBuf::from("/tmp")
        );
        assert_eq!(absolutize(Path::new(".")).unwrap(), cwd.join("."));
        assert_eq!(
            relative_string(Path::new("/repo"), Path::new("/other/file.ts")),
            "/other/file.ts"
        );
    }

    #[test]
    fn build_globset_rejects_invalid_patterns() {
        assert!(build_globset(&["[".to_string()]).is_err());
    }

    #[test]
    fn walk_files_returns_files_and_skips_configured_directories() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::create_dir_all(dir.path().join("node_modules/pkg")).unwrap();
        std::fs::write(dir.path().join("src/b.ts"), "").unwrap();
        std::fs::write(dir.path().join("src/a.ts"), "").unwrap();
        std::fs::write(dir.path().join("node_modules/pkg/app.ts"), "").unwrap();
        let files: Vec<String> = walk_files(dir.path())
            .into_iter()
            .map(|path| relative_string(dir.path(), &path))
            .collect();
        assert_eq!(files, vec!["src/a.ts", "src/b.ts"]);
    }

    #[test]
    fn collect_app_selectors_skips_missing_roots_and_non_source_files() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("web/app")).unwrap();
        std::fs::write(
            dir.path().join("web/app/page.tsx"),
            r#"<button data-testid="save" />"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("web/app/style.css"),
            r#"[data-testid="ignored"] {}"#,
        )
        .unwrap();
        let settings = Settings {
            frontend_root: "web/app".to_string(),
            playwright_config: None,
            test_include: vec![],
            test_exclude: vec![],
            ignore_routes: vec![],
            navigation_helpers: vec![],
            selector_attributes: vec!["data-testid".to_string()],
            selector_roots: vec!["missing".to_string(), "web/app".to_string()],
            selector_include: vec![],
            selector_exclude: vec![],
        };

        let selector_regexes = selectors::compile_selector_regexes(&settings.selector_attributes);
        let selectors = collect_app_selectors(dir.path(), &settings, &selector_regexes).unwrap();
        assert_eq!(selectors.len(), 1);
        assert_eq!(selectors[0].display_value(), "save");
    }

    #[test]
    fn coverage_sort_uses_file_as_tiebreaker() {
        let root = Path::new("/repo");
        let routes = vec![
            Route {
                file: PathBuf::from("/repo/web/app/a/page.tsx"),
                pattern: "/same".to_string(),
            },
            Route {
                file: PathBuf::from("/repo/web/app/b/page.tsx"),
                pattern: "/same".to_string(),
            },
        ];
        let settings = Settings {
            frontend_root: "web/app".to_string(),
            playwright_config: None,
            test_include: vec![],
            test_exclude: vec![],
            ignore_routes: vec![],
            navigation_helpers: vec![],
            selector_attributes: vec!["data-testid".to_string(), "data-pw".to_string()],
            selector_roots: vec!["web/app".to_string()],
            selector_include: vec![],
            selector_exclude: vec![],
        };
        let report = build_coverage(root, &routes, &[], &[], &settings);
        assert_eq!(report.routes[0].file, "web/app/a/page.tsx");
        assert_eq!(report.routes[1].file, "web/app/b/page.tsx");
    }

    #[test]
    fn selector_coverage_sorts_and_counts_uncovered() {
        let root = Path::new("/repo");
        let app_selectors = vec![selectors::AppSelector {
            file: PathBuf::from("/repo/web/app/page.tsx"),
            attribute: "data-testid".to_string(),
            value: selectors::AppSelectorValue::Exact("save".to_string()),
        }];
        let settings = Settings {
            frontend_root: "web/app".to_string(),
            playwright_config: None,
            test_include: vec![],
            test_exclude: vec![],
            ignore_routes: vec![],
            navigation_helpers: vec![],
            selector_attributes: vec!["data-testid".to_string()],
            selector_roots: vec!["web/app".to_string()],
            selector_include: vec![],
            selector_exclude: vec![],
        };
        let report = build_coverage(root, &[], &app_selectors, &[], &settings);
        assert_eq!(report.summary.total_selectors, 1);
        assert_eq!(report.summary.uncovered_selectors, 1);
        assert_eq!(report.selectors[0].file, "web/app/page.tsx");
    }

    #[test]
    fn selector_coverage_sort_uses_value_and_file_tiebreakers() {
        let root = Path::new("/repo");
        let app_selectors = vec![
            selectors::AppSelector {
                file: PathBuf::from("/repo/web/app/b.tsx"),
                attribute: "data-testid".to_string(),
                value: selectors::AppSelectorValue::Exact("same".to_string()),
            },
            selectors::AppSelector {
                file: PathBuf::from("/repo/web/app/a.tsx"),
                attribute: "data-testid".to_string(),
                value: selectors::AppSelectorValue::Exact("same".to_string()),
            },
            selectors::AppSelector {
                file: PathBuf::from("/repo/web/app/c.tsx"),
                attribute: "data-testid".to_string(),
                value: selectors::AppSelectorValue::Exact("zzz".to_string()),
            },
        ];
        let settings = Settings {
            frontend_root: "web/app".to_string(),
            playwright_config: None,
            test_include: vec![],
            test_exclude: vec![],
            ignore_routes: vec![],
            navigation_helpers: vec![],
            selector_attributes: vec!["data-testid".to_string()],
            selector_roots: vec!["web/app".to_string()],
            selector_include: vec![],
            selector_exclude: vec![],
        };
        let report = build_coverage(root, &[], &app_selectors, &[], &settings);
        assert_eq!(report.selectors[0].file, "web/app/a.tsx");
        assert_eq!(report.selectors[1].file, "web/app/b.tsx");
        assert_eq!(report.selectors[2].value, "zzz");
    }

    #[test]
    fn selector_edges_mark_targets_covered() {
        let root = Path::new("/repo");
        let app_selectors = vec![selectors::AppSelector {
            file: PathBuf::from("/repo/web/app/page.tsx"),
            attribute: "data-testid".to_string(),
            value: selectors::AppSelectorValue::Exact("save".to_string()),
        }];
        let edges = vec![Edge::Selector {
            test_file: "tests/e2e/app.spec.ts".to_string(),
            app_file: "web/app/page.tsx".to_string(),
            attribute: "data-testid".to_string(),
            value: "save".to_string(),
            selector: "getByTestId(save)".to_string(),
        }];
        let settings = Settings {
            frontend_root: "web/app".to_string(),
            playwright_config: None,
            test_include: vec![],
            test_exclude: vec![],
            ignore_routes: vec![],
            navigation_helpers: vec![],
            selector_attributes: vec!["data-testid".to_string()],
            selector_roots: vec!["web/app".to_string()],
            selector_include: vec![],
            selector_exclude: vec![],
        };
        let report = build_coverage(root, &[], &app_selectors, &edges, &settings);
        assert_eq!(report.summary.covered_selectors, 1);
        assert_eq!(report.selectors[0].tests, vec!["tests/e2e/app.spec.ts"]);
    }

    #[test]
    fn route_edges_mark_routes_covered() {
        let root = Path::new("/repo");
        let routes = vec![Route {
            file: PathBuf::from("/repo/web/app/users/[id]/page.tsx"),
            pattern: "/users/:id".to_string(),
        }];
        let edges = vec![Edge::Route {
            test_file: "tests/e2e/users.spec.ts".to_string(),
            route_file: "web/app/users/[id]/page.tsx".to_string(),
            route: "/users/:id".to_string(),
            url: "/users/42".to_string(),
        }];
        let settings = Settings {
            frontend_root: "web/app".to_string(),
            playwright_config: None,
            test_include: vec![],
            test_exclude: vec![],
            ignore_routes: vec![],
            navigation_helpers: vec![],
            selector_attributes: vec!["data-testid".to_string()],
            selector_roots: vec!["web/app".to_string()],
            selector_include: vec![],
            selector_exclude: vec![],
        };
        let report = build_coverage(root, &routes, &[], &edges, &settings);
        assert_eq!(report.summary.covered_routes, 1);
        assert_eq!(report.routes[0].urls, vec!["/users/42"]);
    }

    #[test]
    fn analyze_discovers_tests_and_builds_reports() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("web/app")).unwrap();
        std::fs::create_dir_all(dir.path().join("tests/e2e")).unwrap();
        std::fs::write(
            dir.path().join("web/app/page.tsx"),
            r#"export default function Page() { return <main />; }"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("tests/e2e/app.spec.ts"),
            "import { test } from '@playwright/test'; test('home', async ({ page }) => { await page.goto('/'); });",
        )
        .unwrap();
        let settings = Settings {
            frontend_root: "web/app".to_string(),
            playwright_config: None,
            test_include: vec!["tests/**/*.spec.ts".to_string()],
            test_exclude: vec![],
            ignore_routes: vec![],
            navigation_helpers: vec![],
            selector_attributes: vec![],
            selector_roots: vec!["web/app".to_string()],
            selector_include: vec![],
            selector_exclude: vec![],
        };

        let analysis = analyze(dir.path(), &settings).unwrap();
        assert_eq!(analysis.coverage.summary.covered_routes, 1);
        assert_eq!(analysis.edges.edges.len(), 1);
    }

    #[test]
    fn text_printers_cover_routes_and_selectors() {
        let coverage = CoverageReport {
            summary: Summary {
                total_routes: 1,
                covered_routes: 0,
                uncovered_routes: 1,
                total_selectors: 1,
                covered_selectors: 0,
                uncovered_selectors: 1,
            },
            routes: vec![CoverageRoute {
                route: "/missing".to_string(),
                file: "web/app/missing/page.tsx".to_string(),
                covered: false,
                tests: vec![],
                urls: vec![],
            }],
            selectors: vec![CoverageSelector {
                attribute: "data-testid".to_string(),
                value: "missing".to_string(),
                file: "web/app/page.tsx".to_string(),
                covered: false,
                unsupported_dynamic: false,
                tests: vec![],
                selectors: vec![],
            }],
        };
        print_coverage_text(&coverage);

        let edges = EdgeReport {
            edges: vec![
                Edge::Route {
                    test_file: "tests/e2e/app.spec.ts".to_string(),
                    route_file: "web/app/page.tsx".to_string(),
                    route: "/".to_string(),
                    url: "/".to_string(),
                },
                Edge::Selector {
                    test_file: "tests/e2e/app.spec.ts".to_string(),
                    app_file: "web/app/page.tsx".to_string(),
                    attribute: "data-testid".to_string(),
                    value: "save".to_string(),
                    selector: "getByTestId(save)".to_string(),
                },
            ],
        };
        print_edges_text(&edges);
    }
}
