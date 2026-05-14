mod ast;
mod config;
mod matcher;
mod playwright_config;
mod playwright_tests;
mod playwright_urls;
mod routes;
mod selectors;
#[cfg(test)]
mod test_support;

use anyhow::Context;
use anyhow::Result;
use clap::{Parser, Subcommand};
use config::Settings;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use rayon::prelude::*;
use routes::Route;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use walkdir::WalkDir;

#[derive(Parser, Clone)]
#[command(author, version, about)]
pub struct Cli {
    #[arg(long, default_value = ".", global = true)]
    pub root: PathBuf,

    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    #[arg(long, global = true)]
    pub playwright_config: Vec<PathBuf>,

    #[arg(long, global = true)]
    pub project: Option<String>,

    #[arg(long, global = true)]
    pub json: bool,

    #[arg(long, global = true)]
    pub assert_conditional_tests: bool,

    #[arg(long, global = true)]
    pub allow_skipped_tests: bool,

    #[arg(
        long,
        global = true,
        help = "Fail check when exact test ID values are used more than once"
    )]
    pub assert_unique_test_ids: bool,

    #[arg(
        long,
        global = true,
        help = "Fail check when exact HTML id values are used more than once"
    )]
    pub assert_unique_html_ids: bool,

    #[arg(
        long,
        global = true,
        help = "Deprecated: use --assert-unique-test-ids and --assert-unique-html-ids"
    )]
    pub assert_unique_selectors: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Clone)]
pub enum Command {
    Check,
    Edges,
    Related {
        #[arg(required = true, num_args = 1..)]
        files: Vec<PathBuf>,
    },
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
    duplicate_selectors: usize,
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
#[serde(rename_all = "camelCase")]
struct DuplicateSelector {
    attribute: String,
    value: String,
    file: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CoverageReport {
    summary: Summary,
    routes: Vec<CoverageRoute>,
    selectors: Vec<CoverageSelector>,
    duplicate_selectors: Vec<DuplicateSelector>,
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

#[derive(Serialize)]
struct RelatedReport {
    tests: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Analysis {
    coverage: CoverageReport,
    edges: EdgeReport,
}

#[derive(Clone, Copy, Default)]
struct UniqueSelectorPolicy {
    test_ids: bool,
    html_ids: bool,
    aggregate: bool,
    configured_html_id_selector: bool,
}

struct RouteTarget {
    route_file: String,
    pattern: String,
    segments: Vec<String>,
}

struct AppSelectorTarget<'a> {
    selector: &'a selectors::AppSelector,
    app_file: String,
    value: String,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
struct TestProjectContext {
    base_url: Option<String>,
    test_id_attribute: String,
}

struct DiscoveredTestFile {
    path: PathBuf,
    contexts: Vec<TestProjectContext>,
}

struct TestProjectDiscovery {
    context: TestProjectContext,
    test_dir: PathBuf,
    include: GlobSet,
    ignore: GlobSet,
}

struct TestAnalysisContext<'a> {
    root: &'a Path,
    route_index: &'a RouteIndex,
    app_selector_targets: &'a [AppSelectorTarget<'a>],
    selector_index: &'a SelectorIndex<'a>,
    navigation_helpers: &'a [String],
    selector_regexes: &'a selectors::SelectorRegexes,
    test_policy: playwright_tests::TestPolicy,
}

#[derive(Default)]
struct RouteIndex {
    root: Vec<RouteTarget>,
    literal_first: HashMap<String, Vec<RouteTarget>>,
    dynamic_first: Vec<RouteTarget>,
}

#[derive(Default)]
struct SelectorIndex<'a> {
    exact: HashMap<String, HashMap<String, Vec<&'a AppSelectorTarget<'a>>>>,
    by_attribute: HashMap<String, Vec<&'a AppSelectorTarget<'a>>>,
    templates_by_attribute: HashMap<String, Vec<&'a AppSelectorTarget<'a>>>,
}

type SelectorCoverageKey = (String, String, String);
type CoverageLinks = (BTreeSet<String>, BTreeSet<String>);

impl TestProjectContext {
    fn from_project(project: &playwright_config::TestProject) -> Self {
        Self {
            base_url: project.base_url.clone(),
            test_id_attribute: project.test_id_attribute.clone(),
        }
    }
}

impl DiscoveredTestFile {
    fn base_urls(&self) -> Vec<String> {
        let mut urls: Vec<String> = self
            .contexts
            .iter()
            .filter_map(|context| context.base_url.clone())
            .collect();
        urls.sort();
        urls.dedup();
        urls
    }

    fn test_id_attributes(&self) -> Vec<String> {
        let mut attributes: Vec<String> = self
            .contexts
            .iter()
            .map(|context| context.test_id_attribute.clone())
            .collect();
        attributes.sort();
        attributes.dedup();
        attributes
    }
}

pub fn run(cli: Cli) -> Result<ExitCode> {
    let root = absolutize(&cli.root).context("failed to resolve --root")?;
    let settings = config::load_settings(
        &root,
        cli.config.as_deref(),
        &cli.playwright_config,
        cli.project.clone(),
    )?;
    let analysis = analyze_with_policy(
        &root,
        &settings,
        playwright_tests::TestPolicy {
            assert_conditional_tests: cli.assert_conditional_tests,
            allow_skipped_tests: cli.allow_skipped_tests,
        },
        UniqueSelectorPolicy {
            test_ids: cli.assert_unique_test_ids || cli.assert_unique_selectors,
            html_ids: cli.assert_unique_html_ids
                || (cli.assert_unique_selectors && settings.html_ids),
            aggregate: cli.assert_unique_selectors,
            configured_html_id_selector: false,
        },
    )?;
    match cli.command {
        Command::Check => {
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&analysis.coverage)?);
            } else {
                print_coverage_text(&analysis.coverage);
            }
            if analysis.coverage.summary.uncovered_routes > 0
                || analysis.coverage.summary.uncovered_selectors > 0
                || analysis.coverage.summary.duplicate_selectors > 0
            {
                Ok(ExitCode::from(1))
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        Command::Edges => {
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&analysis.edges)?);
            } else {
                print_edges_text(&analysis.edges);
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Related { files } => {
            let related = build_related_report(&root, &analysis.edges.edges, &files);
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&related)?);
            } else {
                print_related_text(&related);
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}

#[cfg(test)]
fn analyze(root: &Path, settings: &Settings) -> Result<Analysis> {
    analyze_with_policy(
        root,
        settings,
        playwright_tests::TestPolicy::default(),
        UniqueSelectorPolicy::default(),
    )
}

fn analyze_with_policy(
    root: &Path,
    settings: &Settings,
    test_policy: playwright_tests::TestPolicy,
    mut unique_selector_policy: UniqueSelectorPolicy,
) -> Result<Analysis> {
    unique_selector_policy.configured_html_id_selector = has_configured_html_id_selector(settings);
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

    let playwright = playwright_config::load_many(
        root,
        &settings.playwright_configs,
        settings.project.as_deref(),
    )?;
    let test_files = discover_test_files(root, settings, &playwright)?;
    let selector_regexes = selectors::compile_selector_regexes_with_html_ids(
        &settings.selector_attributes,
        &settings.component_selector_attributes,
        settings.html_ids,
    );
    let unique_html_id_scan = unique_selector_policy.html_ids && !settings.html_ids;
    let app_selector_regexes = selectors::compile_selector_regexes_with_html_ids(
        &settings.selector_attributes,
        &settings.component_selector_attributes,
        settings.html_ids || unique_html_id_scan,
    );
    let app_selector_occurrences = if settings.selector_attributes.is_empty()
        && settings.component_selector_attributes.is_empty()
        && !settings.html_ids
        && !unique_html_id_scan
    {
        Vec::new()
    } else {
        collect_app_selector_occurrences(root, settings, &app_selector_regexes)?
    };
    let mut app_selectors: Vec<_> = app_selector_occurrences
        .iter()
        .filter(|selector| {
            settings.html_ids
                || unique_selector_policy.configured_html_id_selector
                || selector.attribute != selectors::HTML_ID_ATTRIBUTE
        })
        .cloned()
        .collect();
    app_selectors.sort();
    app_selectors.dedup();
    let route_index = route_index(root, &routes);
    let app_selector_targets = app_selector_targets(root, &app_selectors);
    let selector_index = selector_index(&app_selector_targets);
    let test_analysis = TestAnalysisContext {
        root,
        route_index: &route_index,
        app_selector_targets: &app_selector_targets,
        selector_index: &selector_index,
        navigation_helpers: &settings.navigation_helpers,
        selector_regexes: &selector_regexes,
        test_policy,
    };

    let mut edges: Vec<Edge> = test_files
        .par_iter()
        .try_fold(Vec::new, |mut edges, test_file| -> Result<_> {
            edges.extend(analyze_test_file(test_file, &test_analysis)?);
            Ok(edges)
        })
        .try_reduce(Vec::new, |mut left, mut right| -> Result<_> {
            left.append(&mut right);
            Ok(left)
        })?;
    edges.sort();
    edges.dedup();

    let edge_report = EdgeReport { edges };
    let coverage = build_coverage(
        root,
        &routes,
        &app_selectors,
        &app_selector_occurrences,
        &edge_report.edges,
        settings,
        unique_selector_policy,
    );
    Ok(Analysis {
        coverage,
        edges: edge_report,
    })
}

fn analyze_test_file(
    test_file: &DiscoveredTestFile,
    context: &TestAnalysisContext<'_>,
) -> Result<Vec<Edge>> {
    let source = std::fs::read_to_string(&test_file.path)?;
    let rel_test_file = relative_string(context.root, &test_file.path);
    let mut edges = Vec::new();
    let base_urls = test_file.base_urls();
    let test_id_attributes = test_file.test_id_attributes();

    let (raw_urls, playwright_selectors) =
        ast::with_program(&test_file.path, &source, |program, source| {
            let raw_urls = playwright_urls::extract_playwright_url_occurrences_from_program(
                program,
                source,
                context.navigation_helpers,
            );
            let playwright_selectors = if context.app_selector_targets.is_empty() {
                Vec::new()
            } else {
                selectors::extract_playwright_selector_occurrences_from_program(
                    program,
                    source,
                    context.selector_regexes,
                    &test_id_attributes,
                )
            };
            (raw_urls, playwright_selectors)
        })?;

    for raw_url in raw_urls {
        if !context.test_policy.allows(raw_url.status) {
            continue;
        }
        let Some(url) = normalize_url(&raw_url.value, &base_urls) else {
            continue;
        };
        let ref_segments = matcher::reference_segments(&url);
        let matching_routes: Vec<&RouteTarget> = context
            .route_index
            .candidates(&ref_segments)
            .into_iter()
            .filter(|route| matcher::matches_segments(&ref_segments, &route.segments))
            .collect();
        let Some(best_specificity) = matching_routes
            .iter()
            .map(|route| route_specificity(&route.segments))
            .max()
        else {
            continue;
        };
        for route in matching_routes
            .into_iter()
            .filter(|route| route_specificity(&route.segments) == best_specificity)
        {
            edges.push(Edge::Route {
                test_file: rel_test_file.clone(),
                route_file: route.route_file.clone(),
                route: route.pattern.clone(),
                url: url.clone(),
            });
        }
    }

    if !context.app_selector_targets.is_empty() {
        for playwright_selector in &playwright_selectors {
            if !context.test_policy.allows(playwright_selector.status) {
                continue;
            }
            for app_selector in context.selector_index.matches(&playwright_selector.value) {
                edges.push(Edge::Selector {
                    test_file: rel_test_file.clone(),
                    app_file: app_selector.app_file.clone(),
                    attribute: app_selector.selector.attribute.clone(),
                    value: app_selector.value.clone(),
                    selector: playwright_selector.value.selector.clone(),
                });
            }
        }
    }

    Ok(edges)
}

fn route_index(root: &Path, routes: &[Route]) -> RouteIndex {
    let mut index = RouteIndex::default();
    for route in routes {
        let target = RouteTarget {
            route_file: relative_string(root, &route.file),
            pattern: route.pattern.clone(),
            segments: matcher::pattern_segments(&route.pattern)
                .into_iter()
                .map(str::to_string)
                .collect(),
        };
        match target.segments.first() {
            None => index.root.push(target),
            Some(first) if is_dynamic_pattern_segment(first) => index.dynamic_first.push(target),
            Some(first) => index
                .literal_first
                .entry(first.clone())
                .or_default()
                .push(target),
        }
    }
    index
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

fn selector_index<'a>(targets: &'a [AppSelectorTarget<'a>]) -> SelectorIndex<'a> {
    let mut index = SelectorIndex::default();
    for target in targets {
        if target.selector.unsupported_dynamic() {
            continue;
        }
        index
            .by_attribute
            .entry(target.selector.attribute.clone())
            .or_default()
            .push(target);
        if let selectors::AppSelectorValue::Exact(value) = &target.selector.value {
            index
                .exact
                .entry(target.selector.attribute.clone())
                .or_default()
                .entry(value.clone())
                .or_default()
                .push(target);
        }
        if matches!(
            target.selector.value,
            selectors::AppSelectorValue::Template(_)
        ) {
            index
                .templates_by_attribute
                .entry(target.selector.attribute.clone())
                .or_default()
                .push(target);
        }
    }
    index
}

impl RouteIndex {
    fn candidates<'a>(&'a self, reference_segments: &[&str]) -> Vec<&'a RouteTarget> {
        if reference_segments.is_empty() {
            return self.root.iter().chain(&self.dynamic_first).collect();
        }

        let mut candidates: Vec<&RouteTarget> = self.dynamic_first.iter().collect();
        if let Some(literal) = self.literal_first.get(reference_segments[0]) {
            candidates.extend(literal);
        }
        candidates
    }
}

impl<'a> SelectorIndex<'a> {
    fn matches(
        &'a self,
        playwright_selector: &selectors::PlaywrightSelector,
    ) -> Vec<&'a AppSelectorTarget<'a>> {
        let mut matches = Vec::new();
        if let Some(value) = playwright_selector.exact_value() {
            if let Some(by_value) = self.exact.get(&playwright_selector.attribute) {
                if let Some(exact) = by_value.get(value) {
                    matches.extend(exact.iter().copied());
                }
            }
            let Some(attribute_targets) = self
                .templates_by_attribute
                .get(&playwright_selector.attribute)
            else {
                return matches;
            };
            for target in attribute_targets {
                if target.selector.matches_playwright(playwright_selector) {
                    matches.push(*target);
                }
            }
            return matches;
        }

        if let Some(attribute_targets) = self.by_attribute.get(&playwright_selector.attribute) {
            for target in attribute_targets {
                if target.selector.matches_playwright(playwright_selector) {
                    matches.push(*target);
                }
            }
        }
        matches
    }
}

fn route_specificity(segments: &[String]) -> Vec<u8> {
    let mut specificity: Vec<u8> = segments
        .iter()
        .map(|segment| {
            if segment == "**" {
                0
            } else if segment == "*" {
                1
            } else if segment.starts_with(':') {
                2
            } else {
                3
            }
        })
        .collect();
    specificity.push(4);
    specificity
}

#[cfg(test)]
fn collect_app_selectors(
    root: &Path,
    settings: &Settings,
    selector_regexes: &selectors::SelectorRegexes,
) -> Result<Vec<selectors::AppSelector>> {
    let mut app_selectors = collect_app_selector_occurrences(root, settings, selector_regexes)?;
    app_selectors.sort();
    app_selectors.dedup();

    Ok(app_selectors)
}

fn collect_app_selector_occurrences(
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
        .try_fold(Vec::new, |mut app_selectors, path| -> Result<_> {
            let source = std::fs::read_to_string(path)?;
            app_selectors.extend(selectors::extract_app_selectors_with_regexes(
                path,
                &source,
                selector_regexes,
            )?);
            Ok(app_selectors)
        })
        .try_reduce(Vec::new, |mut left, mut right| -> Result<_> {
            left.append(&mut right);
            Ok(left)
        })?;
    Ok(app_selectors)
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
) -> Result<Vec<DiscoveredTestFile>> {
    let project_discovery = build_project_discovery(root, playwright)?;
    let all_contexts = test_project_contexts(&project_discovery);
    if !settings.test_include.is_empty() {
        let include = build_globset(&settings.test_include)?;
        let exclude = build_globset(&settings.test_exclude)?;
        let mut files = Vec::new();
        for path in walk_files(root).into_iter().filter(|path| {
            let rel = relative_string(root, path);
            include.is_match(&rel) && !exclude.is_match(&rel)
        }) {
            let mut contexts = matching_project_contexts(root, &project_discovery, &path);
            if contexts.is_empty() {
                contexts = all_contexts.clone();
            }
            files.push(DiscoveredTestFile { contexts, path });
        }
        return Ok(files);
    }

    let yaml_exclude = build_globset(&settings.test_exclude)?;
    let mut files: BTreeMap<PathBuf, BTreeSet<TestProjectContext>> = BTreeMap::new();
    let mut projects_by_test_dir: HashMap<PathBuf, Vec<&TestProjectDiscovery>> = HashMap::new();

    for project_discovery in &project_discovery {
        if !project_discovery.test_dir.exists() {
            continue;
        }
        projects_by_test_dir
            .entry(project_discovery.test_dir.clone())
            .or_default()
            .push(project_discovery);
    }

    for (test_dir, projects) in projects_by_test_dir {
        for path in walk_files(&test_dir) {
            let rel_root = relative_string(root, &path);
            if yaml_exclude.is_match(&rel_root) {
                continue;
            }
            let rel_test = relative_string(&test_dir, &path);
            let abs = slash_path(&path);
            let mut contexts_to_add = Vec::new();
            for project_discovery in &projects {
                let included = project_discovery.include.is_match(&rel_root)
                    || project_discovery.include.is_match(&rel_test)
                    || project_discovery.include.is_match(&abs);
                let ignored = project_discovery.ignore.is_match(&rel_root)
                    || project_discovery.ignore.is_match(&rel_test)
                    || project_discovery.ignore.is_match(&abs);
                if included && !ignored {
                    contexts_to_add.push(project_discovery.context.clone());
                }
            }
            if !contexts_to_add.is_empty() {
                files.entry(path).or_default().extend(contexts_to_add);
            }
        }
    }

    Ok(files
        .into_iter()
        .map(|(path, contexts)| DiscoveredTestFile {
            path,
            contexts: contexts.into_iter().collect(),
        })
        .collect())
}

fn build_project_discovery(
    root: &Path,
    playwright: &playwright_config::PlaywrightConfig,
) -> Result<Vec<TestProjectDiscovery>> {
    let mut discovery = Vec::new();
    for project in &playwright.projects {
        discovery.push(TestProjectDiscovery {
            context: TestProjectContext::from_project(project),
            test_dir: project.test_dir(root),
            include: build_globset(&project.test_match)?,
            ignore: build_globset(&project.test_ignore)?,
        });
    }
    Ok(discovery)
}

fn test_project_contexts(projects: &[TestProjectDiscovery]) -> Vec<TestProjectContext> {
    let mut contexts: Vec<TestProjectContext> = projects
        .iter()
        .map(|project| project.context.clone())
        .collect();
    contexts.sort();
    contexts.dedup();
    contexts
}

fn matching_project_contexts(
    root: &Path,
    projects: &[TestProjectDiscovery],
    path: &Path,
) -> Vec<TestProjectContext> {
    let rel_root = relative_string(root, path);
    let mut contexts = BTreeSet::new();
    for project in projects {
        if !path.starts_with(&project.test_dir) {
            continue;
        }

        let rel_test = relative_string(&project.test_dir, path);
        let abs = slash_path(path);
        let included = project.include.is_match(&rel_root)
            || project.include.is_match(&rel_test)
            || project.include.is_match(&abs);
        let ignored = project.ignore.is_match(&rel_root)
            || project.ignore.is_match(&rel_test)
            || project.ignore.is_match(&abs);
        if included && !ignored {
            contexts.insert(project.context.clone());
        }
    }
    contexts.into_iter().collect()
}

fn build_coverage(
    root: &Path,
    routes: &[Route],
    app_selectors: &[selectors::AppSelector],
    app_selector_occurrences: &[selectors::AppSelector],
    edges: &[Edge],
    settings: &Settings,
    unique_selector_policy: UniqueSelectorPolicy,
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
    let duplicate_selectors =
        build_duplicate_selectors(root, app_selector_occurrences, unique_selector_policy);
    let duplicate_selector_count = duplicate_selectors.len();

    CoverageReport {
        summary: Summary {
            total_routes,
            covered_routes,
            uncovered_routes,
            total_selectors,
            covered_selectors,
            uncovered_selectors,
            duplicate_selectors: duplicate_selector_count,
        },
        routes: coverage_routes,
        selectors: coverage_selectors,
        duplicate_selectors,
    }
}

fn build_duplicate_selectors(
    root: &Path,
    app_selectors: &[selectors::AppSelector],
    policy: UniqueSelectorPolicy,
) -> Vec<DuplicateSelector> {
    let mut by_value: BTreeMap<DuplicateSelectorKey<'_>, Vec<&selectors::AppSelector>> =
        BTreeMap::new();
    for selector in app_selectors {
        if let selectors::AppSelectorValue::Exact(value) = &selector.value {
            if policy.aggregate {
                by_value
                    .entry(DuplicateSelectorKey::Aggregate(value.as_str()))
                    .or_default()
                    .push(selector);
            } else if selector.attribute == selectors::HTML_ID_ATTRIBUTE {
                if policy.html_ids || (policy.test_ids && policy.configured_html_id_selector) {
                    by_value
                        .entry(DuplicateSelectorKey::HtmlId(value.as_str()))
                        .or_default()
                        .push(selector);
                }
            } else if policy.test_ids {
                by_value
                    .entry(DuplicateSelectorKey::TestId(value.as_str()))
                    .or_default()
                    .push(selector);
            }
        }
    }

    let mut duplicates = Vec::new();
    for (key, selectors) in by_value {
        if selectors.len() < 2 {
            continue;
        }
        let value = key.value().to_string();
        for selector in selectors {
            duplicates.push(DuplicateSelector {
                attribute: selector.attribute.clone(),
                value: value.clone(),
                file: relative_string(root, &selector.file),
            });
        }
    }
    duplicates.sort_by(|a, b| {
        a.value
            .cmp(&b.value)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.attribute.cmp(&b.attribute))
    });
    duplicates
}

#[derive(Eq, PartialEq, Ord, PartialOrd)]
enum DuplicateSelectorKey<'a> {
    Aggregate(&'a str),
    TestId(&'a str),
    HtmlId(&'a str),
}

impl DuplicateSelectorKey<'_> {
    fn value(&self) -> &str {
        match self {
            Self::Aggregate(value) | Self::TestId(value) | Self::HtmlId(value) => value,
        }
    }
}

fn has_configured_html_id_selector(settings: &Settings) -> bool {
    settings
        .selector_attributes
        .iter()
        .any(|attribute| attribute == selectors::HTML_ID_ATTRIBUTE)
        || settings
            .component_selector_attributes
            .values()
            .any(|attribute| attribute == selectors::HTML_ID_ATTRIBUTE)
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
    println!(
        "Duplicate selectors: {}",
        report.summary.duplicate_selectors
    );

    if report.summary.uncovered_routes == 0
        && report.summary.uncovered_selectors == 0
        && report.summary.duplicate_selectors == 0
    {
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

    if report.summary.duplicate_selectors > 0 {
        println!();
        println!("Duplicate selectors:");
        for selector in &report.duplicate_selectors {
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

fn build_related_report(root: &Path, edges: &[Edge], files: &[PathBuf]) -> RelatedReport {
    let related_files: BTreeSet<String> = files
        .iter()
        .map(|file| related_input_file(root, file))
        .collect();
    let mut tests = BTreeSet::new();

    for edge in edges {
        match edge {
            Edge::Route {
                test_file,
                route_file,
                ..
            } if related_files.contains(route_file) => {
                tests.insert(test_file.clone());
            }
            Edge::Selector {
                test_file,
                app_file,
                ..
            } if related_files.contains(app_file) => {
                tests.insert(test_file.clone());
            }
            _ => {}
        }
    }

    RelatedReport {
        tests: tests.into_iter().collect(),
    }
}

fn related_input_file(root: &Path, file: &Path) -> String {
    if file.is_absolute() {
        return relative_string(root, file);
    }

    let rooted = root.join(file);
    relative_string(root, &rooted)
}

fn print_related_text(report: &RelatedReport) {
    for test in &report.tests {
        println!("{test}");
    }
}

fn normalize_url(raw: &str, base_urls: &[String]) -> Option<String> {
    if raw.starts_with("//") {
        return None;
    }

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

fn is_dynamic_pattern_segment(segment: &str) -> bool {
    segment.starts_with(':') || segment == "*" || segment == "**"
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
    use crate::test_support::fixture_path;

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
    fn normalize_url_handles_edge_cases() {
        let base_urls = vec!["http://localhost:3000".to_string()];
        assert_eq!(normalize_url("//google.com", &base_urls), None);
        assert_eq!(
            normalize_url("http://localhost:3000", &base_urls),
            Some("/".to_string())
        );
        assert_eq!(
            normalize_url("http://localhost:3000/", &base_urls),
            Some("/".to_string())
        );
        assert_eq!(normalize_url("http://other.com", &base_urls), None);
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
    fn compiled_route_matching_handles_edge_segments() {
        assert!(is_dynamic_pattern_segment(":id"));
        assert!(is_dynamic_pattern_segment("*"));
        assert!(is_dynamic_pattern_segment("**"));
        assert!(!is_dynamic_pattern_segment("users"));

        assert_eq!(
            matcher::reference_segments("/users/42/?tab=profile"),
            vec!["users", "42"]
        );
        assert_eq!(
            matcher::pattern_segments("/users/:id"),
            vec!["users", ":id"]
        );
        assert!(matcher::matches_segments(
            &["shop"],
            &["shop".to_string(), "**".to_string()]
        ));
        assert!(!matcher::matches_segments(
            &["shop"],
            &["shop".to_string(), "item".to_string()]
        ));
    }

    #[test]
    fn route_specificity_prefers_earlier_static_segments_and_exact_end() {
        let foo_dynamic: Vec<String> = matcher::pattern_segments("/foo/:id")
            .into_iter()
            .map(str::to_string)
            .collect();
        let dynamic_bar: Vec<String> = matcher::pattern_segments("/:section/bar")
            .into_iter()
            .map(str::to_string)
            .collect();
        let docs_exact: Vec<String> = matcher::pattern_segments("/docs")
            .into_iter()
            .map(str::to_string)
            .collect();
        let docs_catch_all: Vec<String> = matcher::pattern_segments("/docs/**")
            .into_iter()
            .map(str::to_string)
            .collect();

        assert!(route_specificity(&foo_dynamic) > route_specificity(&dynamic_bar));
        assert!(route_specificity(&docs_exact) > route_specificity(&docs_catch_all));
    }

    #[test]
    fn selector_index_matches_exact_template_and_fuzzy_selectors() {
        let root = Path::new("/repo");
        let app_selectors = selectors::extract_app_selectors(
            Path::new("/repo/web/app/page.tsx"),
            r#"
                export function Page({ id }) {
                    return <>
                        <button data-testid="save-button" />
                        <div data-testid={`user-${id}`} />
                        <span data-pw="other" />
                    </>;
                }
            "#,
            &["data-testid".to_string(), "data-pw".to_string()],
            &BTreeMap::new(),
        )
        .unwrap();
        let targets = app_selector_targets(root, &app_selectors);
        let index = selector_index(&targets);

        let exact = selectors::extract_playwright_selectors(
            "await page.getByTestId('user-123');",
            &["data-testid".to_string()],
            &["data-testid".to_string()],
        );
        assert_eq!(index.matches(&exact[0]).len(), 1);

        let fuzzy = selectors::extract_playwright_selectors(
            r#"await page.locator('[data-testid^="save"]');"#,
            &["data-testid".to_string()],
            &["data-testid".to_string()],
        );
        assert_eq!(index.matches(&fuzzy[0]).len(), 1);

        let missing_value = selectors::extract_playwright_selectors(
            r#"await page.locator('[data-testid^="missing"]');"#,
            &["data-testid".to_string()],
            &["data-testid".to_string()],
        );
        assert!(index.matches(&missing_value[0]).is_empty());

        let missing_attribute = selectors::extract_playwright_selectors(
            r#"await page.locator('[data-role^="save"]');"#,
            &["data-role".to_string()],
            &["data-role".to_string()],
        );
        assert!(index.matches(&missing_attribute[0]).is_empty());
    }

    #[test]
    fn build_globset_rejects_invalid_patterns() {
        assert!(build_globset(&["[".to_string()]).is_err());
    }

    #[test]
    fn walk_files_returns_files_and_skips_configured_directories() {
        let root = fixture_path(&["main", "walk-files"]);
        let files: Vec<String> = walk_files(&root)
            .into_iter()
            .map(|path| relative_string(&root, &path))
            .collect();
        assert_eq!(files, vec!["src/a.ts", "src/b.ts"]);
    }

    #[test]
    fn collect_app_selectors_skips_missing_roots_and_non_source_files() {
        let root = fixture_path(&["main", "selector-source"]);
        let settings = Settings {
            frontend_root: "web/app".to_string(),
            playwright_configs: vec![],
            project: None,
            test_include: vec![],
            test_exclude: vec![],
            ignore_routes: vec![],
            navigation_helpers: vec![],
            selector_attributes: vec!["data-testid".to_string()],
            component_selector_attributes: BTreeMap::new(),
            html_ids: false,
            selector_roots: vec!["missing".to_string(), "web/app".to_string()],
            selector_include: vec![],
            selector_exclude: vec![],
        };

        let selector_regexes = selectors::compile_selector_regexes(
            &settings.selector_attributes,
            &settings.component_selector_attributes,
        );
        let selectors = collect_app_selectors(&root, &settings, &selector_regexes).unwrap();
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
            playwright_configs: vec![],
            project: None,
            test_include: vec![],
            test_exclude: vec![],
            ignore_routes: vec![],
            navigation_helpers: vec![],
            selector_attributes: vec!["data-testid".to_string(), "data-pw".to_string()],
            component_selector_attributes: BTreeMap::new(),
            html_ids: false,
            selector_roots: vec!["web/app".to_string()],
            selector_include: vec![],
            selector_exclude: vec![],
        };
        let report = build_coverage(
            root,
            &routes,
            &[],
            &[],
            &[],
            &settings,
            UniqueSelectorPolicy::default(),
        );
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
            playwright_configs: vec![],
            project: None,
            test_include: vec![],
            test_exclude: vec![],
            ignore_routes: vec![],
            navigation_helpers: vec![],
            selector_attributes: vec!["data-testid".to_string()],
            component_selector_attributes: BTreeMap::new(),
            html_ids: false,
            selector_roots: vec!["web/app".to_string()],
            selector_include: vec![],
            selector_exclude: vec![],
        };
        let report = build_coverage(
            root,
            &[],
            &app_selectors,
            &app_selectors,
            &[],
            &settings,
            UniqueSelectorPolicy::default(),
        );
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
            playwright_configs: vec![],
            project: None,
            test_include: vec![],
            test_exclude: vec![],
            ignore_routes: vec![],
            navigation_helpers: vec![],
            selector_attributes: vec!["data-testid".to_string()],
            component_selector_attributes: BTreeMap::new(),
            html_ids: false,
            selector_roots: vec!["web/app".to_string()],
            selector_include: vec![],
            selector_exclude: vec![],
        };
        let report = build_coverage(
            root,
            &[],
            &app_selectors,
            &app_selectors,
            &[],
            &settings,
            UniqueSelectorPolicy::default(),
        );
        assert_eq!(report.selectors[0].file, "web/app/a.tsx");
        assert_eq!(report.selectors[1].file, "web/app/b.tsx");
        assert_eq!(report.selectors[2].value, "zzz");
    }

    #[test]
    fn duplicate_selector_report_includes_exact_values_only() {
        let root = Path::new("/repo");
        let app_selectors = vec![
            selectors::AppSelector {
                file: PathBuf::from("/repo/web/app/b.tsx"),
                attribute: "data-pw".to_string(),
                value: selectors::AppSelectorValue::Exact("same".to_string()),
            },
            selectors::AppSelector {
                file: PathBuf::from("/repo/web/app/b.tsx"),
                attribute: "data-pw".to_string(),
                value: selectors::AppSelectorValue::Exact("same".to_string()),
            },
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
                value: selectors::AppSelectorValue::Unsupported("id".to_string()),
            },
            selectors::AppSelector {
                file: PathBuf::from("/repo/web/app/d.tsx"),
                attribute: "data-testid".to_string(),
                value: selectors::AppSelectorValue::Exact("unique".to_string()),
            },
        ];

        let duplicates = build_duplicate_selectors(
            root,
            &app_selectors,
            UniqueSelectorPolicy {
                test_ids: true,
                html_ids: false,
                ..UniqueSelectorPolicy::default()
            },
        );
        assert_eq!(duplicates.len(), 4);
        assert_eq!(duplicates[0].file, "web/app/a.tsx");
        assert_eq!(duplicates[1].attribute, "data-pw");
        assert_eq!(duplicates[2].attribute, "data-pw");
        assert_eq!(duplicates[3].attribute, "data-testid");
    }

    #[test]
    fn duplicate_selector_report_keeps_html_ids_separate_from_test_ids() {
        let root = Path::new("/repo");
        let app_selectors = vec![
            selectors::AppSelector {
                file: PathBuf::from("/repo/web/app/a.tsx"),
                attribute: "data-testid".to_string(),
                value: selectors::AppSelectorValue::Exact("same".to_string()),
            },
            selectors::AppSelector {
                file: PathBuf::from("/repo/web/app/b.tsx"),
                attribute: "id".to_string(),
                value: selectors::AppSelectorValue::Exact("same".to_string()),
            },
        ];

        let duplicates = build_duplicate_selectors(
            root,
            &app_selectors,
            UniqueSelectorPolicy {
                test_ids: true,
                html_ids: true,
                ..UniqueSelectorPolicy::default()
            },
        );
        assert!(duplicates.is_empty());
    }

    #[test]
    fn deprecated_duplicate_selector_report_preserves_aggregate_grouping() {
        let root = Path::new("/repo");
        let app_selectors = vec![
            selectors::AppSelector {
                file: PathBuf::from("/repo/web/app/a.tsx"),
                attribute: "data-testid".to_string(),
                value: selectors::AppSelectorValue::Exact("same".to_string()),
            },
            selectors::AppSelector {
                file: PathBuf::from("/repo/web/app/b.tsx"),
                attribute: "id".to_string(),
                value: selectors::AppSelectorValue::Exact("same".to_string()),
            },
        ];

        let duplicates = build_duplicate_selectors(
            root,
            &app_selectors,
            UniqueSelectorPolicy {
                test_ids: true,
                html_ids: true,
                aggregate: true,
                ..UniqueSelectorPolicy::default()
            },
        );
        assert_eq!(duplicates.len(), 2);
    }

    #[test]
    fn configured_html_id_selectors_count_as_test_ids_for_uniqueness() {
        let root = Path::new("/repo");
        let app_selectors = vec![
            selectors::AppSelector {
                file: PathBuf::from("/repo/web/app/a.tsx"),
                attribute: "id".to_string(),
                value: selectors::AppSelectorValue::Exact("same".to_string()),
            },
            selectors::AppSelector {
                file: PathBuf::from("/repo/web/app/b.tsx"),
                attribute: "id".to_string(),
                value: selectors::AppSelectorValue::Exact("same".to_string()),
            },
        ];

        let duplicates = build_duplicate_selectors(
            root,
            &app_selectors,
            UniqueSelectorPolicy {
                test_ids: true,
                configured_html_id_selector: true,
                ..UniqueSelectorPolicy::default()
            },
        );
        assert_eq!(duplicates.len(), 2);
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
            playwright_configs: vec![],
            project: None,
            test_include: vec![],
            test_exclude: vec![],
            ignore_routes: vec![],
            navigation_helpers: vec![],
            selector_attributes: vec!["data-testid".to_string()],
            component_selector_attributes: BTreeMap::new(),
            html_ids: false,
            selector_roots: vec!["web/app".to_string()],
            selector_include: vec![],
            selector_exclude: vec![],
        };
        let report = build_coverage(
            root,
            &[],
            &app_selectors,
            &app_selectors,
            &edges,
            &settings,
            UniqueSelectorPolicy::default(),
        );
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
            playwright_configs: vec![],
            project: None,
            test_include: vec![],
            test_exclude: vec![],
            ignore_routes: vec![],
            navigation_helpers: vec![],
            selector_attributes: vec!["data-testid".to_string()],
            component_selector_attributes: BTreeMap::new(),
            html_ids: false,
            selector_roots: vec!["web/app".to_string()],
            selector_include: vec![],
            selector_exclude: vec![],
        };
        let report = build_coverage(
            root,
            &routes,
            &[],
            &[],
            &edges,
            &settings,
            UniqueSelectorPolicy::default(),
        );
        assert_eq!(report.summary.covered_routes, 1);
        assert_eq!(report.routes[0].urls, vec!["/users/42"]);
    }

    #[test]
    fn analyze_discovers_tests_and_builds_reports() {
        let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        root.extend(["tests", "fixtures", "covered"]);
        let settings = Settings {
            frontend_root: "web/app".to_string(),
            playwright_configs: vec![],
            project: None,
            test_include: vec![],
            test_exclude: vec![],
            ignore_routes: vec![],
            navigation_helpers: vec![],
            selector_attributes: vec!["data-testid".to_string()],
            component_selector_attributes: BTreeMap::new(),
            html_ids: false,
            selector_roots: vec!["web/app".to_string()],
            selector_include: vec![],
            selector_exclude: vec![],
        };

        let analysis = analyze(&root, &settings).unwrap();
        assert!(!analysis.coverage.routes.is_empty());
        assert!(!analysis.edges.edges.is_empty());

        // Test run() with Check - fails because it's uncovered
        let run_root = root.join("web");
        let cli = Cli {
            root: run_root.clone(),
            config: None,
            playwright_config: vec![],
            project: None,
            json: false,
            assert_conditional_tests: false,
            allow_skipped_tests: false,
            assert_unique_test_ids: false,
            assert_unique_html_ids: false,
            assert_unique_selectors: false,
            command: Command::Check,
        };
        assert_eq!(run(cli.clone()).unwrap(), ExitCode::from(1));

        // Test run() with JSON
        let mut cli_json = cli.clone();
        cli_json.json = true;
        assert_eq!(run(cli_json).unwrap(), ExitCode::from(1));

        // Test run() with Edges - always SUCCESS
        let mut cli_edges = cli.clone();
        cli_edges.command = Command::Edges;
        assert_eq!(run(cli_edges).unwrap(), ExitCode::SUCCESS);

        // Test run() with Related - always SUCCESS
        let mut cli_related = cli.clone();
        cli_related.command = Command::Related {
            files: vec![PathBuf::from("app/page.tsx")],
        };
        assert_eq!(run(cli_related).unwrap(), ExitCode::SUCCESS);

        // Test with unique selectors and html ids
        let mut cli_unique = cli.clone();
        cli_unique.assert_unique_selectors = true;
        cli_unique.assert_unique_html_ids = true;
        assert_eq!(run(cli_unique).unwrap(), ExitCode::from(1));

        // Coverage for printers
        print_edges_text(&analysis.edges);
        let related = build_related_report(
            &root,
            &analysis.edges.edges,
            &[PathBuf::from("web/app/page.tsx")],
        );
        print_related_text(&related);
        let _ = serde_json::to_string_pretty(&analysis).unwrap();
    }

    #[test]
    fn run_check_success_on_empty_app() {
        let root = fixture_path(&["config", "missing-default"]);
        let cli = Cli {
            root,
            config: None,
            playwright_config: vec![],
            project: None,
            json: false,
            assert_conditional_tests: false,
            allow_skipped_tests: false,
            assert_unique_test_ids: false,
            assert_unique_html_ids: false,
            assert_unique_selectors: false,
            command: Command::Check,
        };
        let _ = run(cli);
    }

    #[test]
    fn run_errors_on_invalid_root() {
        let cli = Cli {
            root: PathBuf::from("/non-existent-path-12345/non-existent-child"),
            config: None,
            playwright_config: vec![],
            project: None,
            json: false,
            assert_conditional_tests: false,
            allow_skipped_tests: false,
            assert_unique_test_ids: false,
            assert_unique_html_ids: false,
            assert_unique_selectors: false,
            command: Command::Check,
        };
        assert!(run(cli).is_err());
    }

    #[test]
    fn run_errors_on_empty_routes() {
        let root = fixture_path(&["main", "empty-app"]);
        let cli = Cli {
            root,
            config: None,
            playwright_config: vec![],
            project: None,
            json: false,
            assert_conditional_tests: false,
            allow_skipped_tests: false,
            assert_unique_test_ids: false,
            assert_unique_html_ids: false,
            assert_unique_selectors: false,
            command: Command::Check,
        };
        assert!(run(cli).is_err());
    }

    #[test]
    fn run_errors_on_missing_playwright_config() {
        let root = fixture_path(&["covered"]);
        let cli = Cli {
            root: root.clone(),
            config: None,
            playwright_config: vec![root.join("missing.config.ts")],
            project: None,
            json: false,
            assert_conditional_tests: false,
            allow_skipped_tests: false,
            assert_unique_test_ids: false,
            assert_unique_html_ids: false,
            assert_unique_selectors: false,
            command: Command::Check,
        };
        assert!(run(cli).is_err());
    }

    #[test]
    fn discover_test_files_walks_shared_project_test_dir_once() {
        let root = fixture_path(&["main", "analyze-basic"]);
        let settings = Settings {
            frontend_root: "web/app".to_string(),
            playwright_configs: vec![],
            project: None,
            test_include: vec![],
            test_exclude: vec![],
            ignore_routes: vec![],
            navigation_helpers: vec![],
            selector_attributes: vec![],
            component_selector_attributes: BTreeMap::new(),
            html_ids: false,
            selector_roots: vec!["web/app".to_string()],
            selector_include: vec![],
            selector_exclude: vec![],
        };
        let playwright = playwright_config::PlaywrightConfig {
            name: None,
            projects: vec![
                playwright_config::TestProject {
                    config_dir: root.clone(),
                    test_dir: "tests".to_string(),
                    test_match: vec!["**/*.spec.ts".to_string()],
                    test_ignore: vec![],
                    base_url: Some("http://localhost:3000".to_string()),
                    test_id_attribute: "data-testid".to_string(),
                },
                playwright_config::TestProject {
                    config_dir: root.clone(),
                    test_dir: "tests".to_string(),
                    test_match: vec!["**/*.spec.ts".to_string()],
                    test_ignore: vec![],
                    base_url: Some("http://localhost:4000".to_string()),
                    test_id_attribute: "data-pw".to_string(),
                },
            ],
        };

        let files = discover_test_files(&root, &settings, &playwright).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].contexts.len(), 2);
    }

    #[test]
    fn discover_test_files_applies_yaml_exclude_before_project_matching() {
        let root = fixture_path(&["main", "analyze-basic"]);
        let settings = Settings {
            frontend_root: "web/app".to_string(),
            playwright_configs: vec![],
            project: None,
            test_include: vec![],
            test_exclude: vec!["tests/**".to_string()],
            ignore_routes: vec![],
            navigation_helpers: vec![],
            selector_attributes: vec![],
            component_selector_attributes: BTreeMap::new(),
            html_ids: false,
            selector_roots: vec!["web/app".to_string()],
            selector_include: vec![],
            selector_exclude: vec![],
        };
        let playwright = playwright_config::PlaywrightConfig {
            name: None,
            projects: vec![playwright_config::TestProject {
                config_dir: root.clone(),
                test_dir: "tests".to_string(),
                test_match: vec!["**/*.spec.ts".to_string()],
                test_ignore: vec![],
                base_url: None,
                test_id_attribute: "data-testid".to_string(),
            }],
        };

        let files = discover_test_files(&root, &settings, &playwright).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn analyze_surfaces_parser_errors() {
        let root = fixture_path(&["main", "invalid-test-source"]);
        let settings = Settings {
            frontend_root: "web/app".to_string(),
            playwright_configs: vec![],
            project: None,
            test_include: vec!["tests/**/*.spec.ts".to_string()],
            test_exclude: vec![],
            ignore_routes: vec![],
            navigation_helpers: vec![],
            selector_attributes: vec![],
            component_selector_attributes: BTreeMap::new(),
            html_ids: false,
            selector_roots: vec!["web/app".to_string()],
            selector_include: vec![],
            selector_exclude: vec![],
        };

        let err = analyze(&root, &settings).err().unwrap();
        assert!(err.to_string().contains("failed to parse"));

        let root = fixture_path(&["main", "invalid-selector-source"]);
        let settings = Settings {
            frontend_root: "web/app".to_string(),
            playwright_configs: vec![],
            project: None,
            test_include: vec![],
            test_exclude: vec![],
            ignore_routes: vec![],
            navigation_helpers: vec![],
            selector_attributes: vec!["data-testid".to_string()],
            component_selector_attributes: BTreeMap::new(),
            html_ids: false,
            selector_roots: vec!["web/app".to_string()],
            selector_include: vec![],
            selector_exclude: vec![],
        };
        let selector_regexes = selectors::compile_selector_regexes(
            &settings.selector_attributes,
            &settings.component_selector_attributes,
        );
        let err = collect_app_selectors(&root, &settings, &selector_regexes)
            .err()
            .unwrap();
        assert!(err.to_string().contains("failed to parse"));
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
                duplicate_selectors: 1,
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
            duplicate_selectors: vec![DuplicateSelector {
                attribute: "data-testid".to_string(),
                value: "missing".to_string(),
                file: "web/app/other.tsx".to_string(),
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

    #[test]
    fn related_report_matches_route_and_selector_edges() {
        let root = Path::new("/repo");
        let edges = vec![
            Edge::Route {
                test_file: "tests/e2e/route.spec.ts".to_string(),
                route_file: "web/app/page.tsx".to_string(),
                route: "/".to_string(),
                url: "/".to_string(),
            },
            Edge::Selector {
                test_file: "tests/e2e/selector.spec.ts".to_string(),
                app_file: "web/app/components/save.tsx".to_string(),
                attribute: "data-testid".to_string(),
                value: "save".to_string(),
                selector: "getByTestId(save)".to_string(),
            },
        ];
        let report = build_related_report(
            root,
            &edges,
            &[
                PathBuf::from("/repo/web/app/page.tsx"),
                PathBuf::from("./web/app/components/save.tsx"),
            ],
        );

        assert_eq!(
            report.tests,
            vec!["tests/e2e/route.spec.ts", "tests/e2e/selector.spec.ts"]
        );
        print_related_text(&report);
    }
}
