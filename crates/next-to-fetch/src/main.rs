use anyhow::Result;
use clap::Parser;
use no_mistakes_core::ast;
use no_mistakes_core::config;
use no_mistakes_core::routes;
use oxc_ast::ast::{
    Argument, CallExpression, ExportNamedDeclaration, Expression, ImportDeclarationSpecifier,
    ImportOrExportKind, Statement,
};
use oxc_ast_visit::{walk, Visit};
use oxc_span::GetSpan;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,

    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[arg(long, global = true)]
    json: bool,

    #[arg(help = "Specific routes or files to analyze")]
    targets: Vec<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RootConfig {
    #[serde(flatten)]
    legacy: FileConfig,
    next_to_fetch: Option<FileConfig>,
}

#[derive(Default, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
struct FileConfig {
    frontend_root: Option<String>,
}

#[derive(Serialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
struct FetchOccurrence {
    path: String,
    raw_path: String,
    method: String,
    file: String,
    line: usize,
    side: FetchSide,
    #[serde(rename = "rsc")]
    rsc: bool,
    cached: bool,
    cache_kind: CacheKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    cached_function: Option<String>,
    dynamic: bool,
    unsupported: bool,
}

#[derive(Debug, Eq, PartialEq)]
struct UrlExtraction {
    path: String,
    raw_path: String,
    is_dynamic: bool,
    is_unsupported: bool,
}

#[derive(Serialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
enum FetchSide {
    Client,
    Server,
}

#[derive(Serialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "kebab-case")]
#[allow(dead_code)]
enum CacheKind {
    None,
    FetchCache,
    FetchNextRevalidate,
    FetchNextTags,
    ReactCache,
    Cache,
    UnstableCache,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RouteReport {
    route: String,
    file: String,
    api_calls: Vec<FetchOccurrence>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct FinalReport {
    summary: Summary,
    routes: Vec<RouteReport>,
    duplicates: Vec<DuplicateApiCall>,
    unsupported: Vec<UnsupportedApiCall>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct Summary {
    total_routes: usize,
    routes_with_api_calls: usize,
    total_api_calls: usize,
    unique_api_calls: usize,
    duplicate_api_calls: usize,
    dynamic_api_calls: usize,
    cached_api_calls: usize,
    client_api_calls: usize,
    server_api_calls: usize,
    rsc_api_calls: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DuplicateApiCall {
    key: String,
    count: usize,
    occurrences: Vec<ApiCallOccurrence>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiCallOccurrence {
    route: String,
    file: String,
    line: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UnsupportedApiCall {
    route: String,
    file: String,
    line: usize,
    reason: String,
    raw_path: String,
}

#[derive(Clone)]
struct TargetSpec {
    raw: String,
    file: Option<PathBuf>,
}

struct Cache {
    files: HashMap<(PathBuf, bool, bool), CachedFile>,
    imports: HashMap<PathBuf, Vec<PathBuf>>,
}

#[derive(Clone)]
struct CachedFile {
    is_client: bool,
    fetches: Vec<FetchOccurrence>,
}

struct FetchVisitor<'a> {
    source: &'a str,
    file: String,
    fetches: Vec<FetchOccurrence>,
    is_client: bool,
    is_route_handler: bool,
    cached_function: Option<String>,
    cached_kind: Option<CacheKind>,
}

impl<'a> Visit<'a> for FetchVisitor<'a> {
    fn visit_call_expression(&mut self, expr: &CallExpression<'a>) {
        if let Some((wrapper_name, cached_kind)) = cache_wrapper_name(expr) {
            let previous_cached_function = self.cached_function.clone();
            let previous_cached_kind = self.cached_kind.clone();
            self.cached_function =
                infer_cached_wrapper_name(self.source, expr).or(Some(wrapper_name));
            self.cached_kind = Some(cached_kind);
            walk::walk_call_expression(self, expr);
            self.cached_function = previous_cached_function;
            self.cached_kind = previous_cached_kind;
            return;
        }

        if let Expression::Identifier(ident) = &expr.callee {
            if ident.name == "fetch" {
                let mut method = "GET".to_string();
                let mut cached = false;
                let mut cache_kind = CacheKind::None;
                let line = self.source[..expr.span().start as usize].lines().count() + 1;

                let (path, raw_path, is_dynamic, is_unsupported) =
                    if let Some(arg) = expr.arguments.first() {
                        let result = extract_url_from_argument(arg, self.source);
                        (
                            result.path,
                            result.raw_path,
                            result.is_dynamic,
                            result.is_unsupported,
                        )
                    } else {
                        ("unknown".to_string(), "unknown".to_string(), true, true)
                    };

                if let Some(cached_kind) = &self.cached_kind {
                    cached = true;
                    cache_kind = cached_kind.clone();
                }

                if let Some(Argument::ObjectExpression(obj)) = expr.arguments.get(1) {
                    for prop in &obj.properties {
                        if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) = prop {
                            if let Some(name) = p.key.static_name() {
                                if name.as_ref() == "method" {
                                    if let Expression::StringLiteral(s) = &p.value {
                                        method = s.value.to_string();
                                    }
                                }
                            }
                        }
                    }
                    let (seen_cached, seen_cache_kind) = extract_fetch_cache_options(obj);
                    if !cached {
                        cached = seen_cached;
                        cache_kind = seen_cache_kind;
                    }
                }

                let side = if self.is_client {
                    FetchSide::Client
                } else {
                    FetchSide::Server
                };
                self.fetches.push(FetchOccurrence {
                    path: path.clone(),
                    raw_path,
                    method,
                    file: self.file.clone(),
                    line,
                    side,
                    rsc: !self.is_client && !self.is_route_handler,
                    cached,
                    cache_kind,
                    cached_function: self.cached_function.clone(),
                    dynamic: is_dynamic,
                    unsupported: is_unsupported,
                });
            }
        }
        walk::walk_call_expression(self, expr);
    }
}

fn infer_cached_wrapper_name(source: &str, expr: &CallExpression<'_>) -> Option<String> {
    let statement_start = source[..expr.span().start as usize]
        .rfind('\n')
        .map_or(0, |idx| idx + 1);
    let assignment = source[statement_start..expr.span().start as usize].trim_end();
    let equal_sign = assignment.rfind('=')?;

    let lhs = assignment[..equal_sign].trim_end();

    let mut cursor = lhs.len();
    let end = cursor;
    while cursor > 0 {
        let ch = lhs[..cursor].chars().last()?;
        if is_identifier_char(ch) {
            cursor -= ch.len_utf8();
        } else {
            break;
        }
    }
    if cursor == end {
        return None;
    }

    let name = &lhs[cursor..end];
    if cursor > 0
        && lhs[..cursor]
            .chars()
            .last()
            .is_some_and(|ch| ch == '.' || ch == '?' || ch == ':' || ch == ')' || ch == ']')
    {
        return None;
    }
    if name
        .chars()
        .next()
        .is_some_and(|char| char.is_ascii_alphabetic() || char == '_' || char == '$')
    {
        Some(name.to_string())
    } else {
        None
    }
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '$'
}

fn cache_wrapper_name(expr: &CallExpression<'_>) -> Option<(String, CacheKind)> {
    let Expression::Identifier(identifier) = &expr.callee else {
        return None;
    };
    match identifier.name.as_ref() {
        "cache" => Some((identifier.name.to_string(), CacheKind::ReactCache)),
        "unstable_cache" => Some((identifier.name.to_string(), CacheKind::UnstableCache)),
        _ => None,
    }
}

fn extract_fetch_cache_options(obj: &oxc_ast::ast::ObjectExpression<'_>) -> (bool, CacheKind) {
    let mut cached = false;
    let mut cache_kind = CacheKind::None;

    for property in &obj.properties {
        let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) = property else {
            continue;
        };
        let Some(name) = property.key.static_name() else {
            continue;
        };

        match name.as_ref() {
            "cache" => {
                if let Expression::StringLiteral(value) = &property.value {
                    if value.value == "force-cache" {
                        cached = true;
                        cache_kind = CacheKind::FetchCache;
                    }
                }
            }
            "next" => {
                if let Expression::ObjectExpression(next_obj) = &property.value {
                    for next_property in &next_obj.properties {
                        let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(next_property) =
                            next_property
                        else {
                            continue;
                        };
                        let Some(next_name) = next_property.key.static_name() else {
                            continue;
                        };
                        match next_name.as_ref() {
                            "revalidate" => match &next_property.value {
                                Expression::NumericLiteral(value) if value.value > 0.0 => {
                                    cached = true;
                                    cache_kind = CacheKind::FetchNextRevalidate;
                                }
                                _ => {}
                            },
                            "tags" => {
                                cached = true;
                                cache_kind = CacheKind::FetchNextTags;
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
    (cached, cache_kind)
}

fn extract_url_from_argument(arg: &Argument, source: &str) -> UrlExtraction {
    match arg {
        Argument::StringLiteral(s) => UrlExtraction {
            path: s.value.to_string(),
            raw_path: s.value.to_string(),
            is_dynamic: false,
            is_unsupported: false,
        },
        Argument::TemplateLiteral(t) => {
            let is_dynamic = !t.expressions.is_empty();
            UrlExtraction {
                path: ast::template_literal_text(t, source),
                raw_path: source_text(t.span().start as usize, t.span().end as usize, source)
                    .unwrap_or_else(|| "dynamic".to_string()),
                is_dynamic,
                is_unsupported: is_dynamic,
            }
        }
        _ => UrlExtraction {
            path: "dynamic".to_string(),
            raw_path: source_text(arg.span().start as usize, arg.span().end as usize, source)
                .unwrap_or_else(|| "dynamic".to_string()),
            is_dynamic: true,
            is_unsupported: true,
        },
    }
}

fn source_text(start: usize, end: usize, source: &str) -> Option<String> {
    if start > end || end > source.len() {
        return None;
    }

    if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
        return None;
    }

    Some(source[start..end].trim().to_string())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let root = std::env::current_dir()?.join(&cli.root);
    if !root.exists() {
        anyhow::bail!("root directory does not exist: {}", root.display());
    }
    let stems = [".no-mistakes", ".next-to-fetch"];
    let root_config: RootConfig = config::load_config(&root, cli.config.as_deref(), &stems)?;
    let file_config = root_config.next_to_fetch.unwrap_or(root_config.legacy);

    let frontend_root_name = file_config
        .frontend_root
        .unwrap_or_else(|| "app".to_string());
    let frontend_root = root.join(&frontend_root_name);
    if !frontend_root.exists() {
        anyhow::bail!(
            "frontend root directory does not exist: {}",
            frontend_root.display()
        );
    }
    let stems = ["page", "route"];
    let all_routes = routes::collect_routes(&frontend_root, &stems)?;

    let mut cache = Cache {
        files: HashMap::new(),
        imports: HashMap::new(),
    };
    let target_specs = cli
        .targets
        .iter()
        .map(|target| TargetSpec {
            raw: target.clone(),
            file: resolve_target_file(&root, target).ok(),
        })
        .collect::<Vec<_>>();
    let mut reports = Vec::new();
    let mut matched_targets = HashSet::new();

    for route in all_routes {
        let route_is_page = route.file.file_stem().and_then(|s| s.to_str()) == Some("page");
        let wrapper_files = if route_is_page {
            collect_layout_chain_files(&route.file, &frontend_root)
                .into_iter()
                .filter_map(|path| path.canonicalize().ok())
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        let mut matched = target_specs.is_empty();
        'target_match: for target in &target_specs {
            if route_matches_target(&route.pattern, &target.raw) {
                matched = true;
                matched_targets.insert(target.raw.clone());
                continue;
            }

            if let Some(target_file) = &target.file {
                let mut visited_targets = HashSet::new();
                if route_reaches_target(
                    &route.file,
                    target_file,
                    &mut visited_targets,
                    &mut cache.imports,
                )? {
                    matched = true;
                    matched_targets.insert(target.raw.clone());
                    continue 'target_match;
                }

                let mut wrapper_file_matches = false;
                for wrapper_file in &wrapper_files {
                    if wrapper_file == target_file {
                        wrapper_file_matches = true;
                        break;
                    }

                    if collect_imports(wrapper_file, &mut cache.imports)?
                        .iter()
                        .any(|import| import == target_file)
                    {
                        wrapper_file_matches = true;
                        break;
                    }
                }

                if wrapper_file_matches {
                    matched = true;
                    matched_targets.insert(target.raw.clone());
                    continue 'target_match;
                }
            }
        }

        let mut visited = HashSet::new();
        let mut fetches = Vec::new();

        if !matched {
            continue;
        }

        let route_is_route_handler = is_route_handler_file(&route.file);
        // Analyze the page/route file itself
        let _route_is_client = analyze_file(
            &route.file,
            &root,
            &mut visited,
            &mut fetches,
            &mut cache,
            false,
            route_is_route_handler,
        )?;

        // Traverse up and find parent layouts/loadings if it's a page (UI)
        if route_is_page {
            let mut current = route.file.parent();
            while let Some(parent) = current {
                if !parent.starts_with(&frontend_root) {
                    break;
                }

                for stem in ["layout", "loading", "error", "not-found", "template"] {
                    for ext in ["tsx", "ts", "jsx", "js"] {
                        let layout_file = parent.join(format!("{stem}.{ext}"));
                        if layout_file.exists() {
                            analyze_file(
                                &layout_file,
                                &root,
                                &mut visited,
                                &mut fetches,
                                &mut cache,
                                false,
                                route_is_route_handler,
                            )?;
                        }
                    }
                }
                current = parent.parent();
            }
        }

        fetches.sort();

        reports.push(RouteReport {
            route: route.pattern,
            file: relative_string(&root, &route.file),
            api_calls: fetches,
        });
    }

    if !target_specs.is_empty() && matched_targets.len() < target_specs.len() {
        let unmatched: Vec<_> = cli
            .targets
            .iter()
            .filter(|t| !matched_targets.contains(*t))
            .collect();
        eprintln!("Error: targets not found: {:?}", unmatched);
        std::process::exit(2);
    }

    let total_routes = reports.len();
    let routes_with_api_calls = reports
        .iter()
        .filter(|route| !route.api_calls.is_empty())
        .count();

    let mut duplicate_key_map: HashMap<(String, String, FetchSide, bool), Vec<ApiCallOccurrence>> =
        HashMap::new();
    let mut unique_api_calls = HashSet::new();
    let mut dynamic_api_calls = 0usize;
    let mut cached_api_calls = 0usize;
    let mut client_api_calls = 0usize;
    let mut server_api_calls = 0usize;
    let mut rsc_api_calls = 0usize;
    let mut unsupported = Vec::new();

    for route in &reports {
        for api_call in &route.api_calls {
            let key = (
                api_call.method.clone(),
                api_call.path.clone(),
                api_call.side.clone(),
                api_call.rsc,
            );
            duplicate_key_map
                .entry(key)
                .or_default()
                .push(ApiCallOccurrence {
                    route: route.route.clone(),
                    file: api_call.file.clone(),
                    line: api_call.line,
                });

            unique_api_calls.insert((
                api_call.method.clone(),
                api_call.path.clone(),
                api_call.side.clone(),
            ));

            if api_call.dynamic {
                dynamic_api_calls += 1;
                unsupported.push(UnsupportedApiCall {
                    route: route.route.clone(),
                    file: api_call.file.clone(),
                    line: api_call.line,
                    reason: "dynamic-path".to_string(),
                    raw_path: api_call.raw_path.clone(),
                });
            }
            if api_call.cached {
                cached_api_calls += 1;
            }
            match api_call.side {
                FetchSide::Client => client_api_calls += 1,
                FetchSide::Server => server_api_calls += 1,
            }
            if api_call.rsc {
                rsc_api_calls += 1;
            }
        }
    }

    let mut duplicates = Vec::new();
    for ((method, path, side, rsc), occurrences) in duplicate_key_map {
        if occurrences.len() > 1 {
            duplicates.push(DuplicateApiCall {
                key: format!(
                    "{method} {path} {} {}",
                    match side {
                        FetchSide::Client => "client",
                        FetchSide::Server => "server",
                    },
                    if rsc { "rsc" } else { "non-rsc" }
                ),
                count: occurrences.len(),
                occurrences,
            });
        }
    }

    let duplicate_api_calls: usize = duplicates
        .iter()
        .map(|entry| entry.count.saturating_sub(1))
        .sum();

    let mut final_report = FinalReport {
        summary: Summary {
            total_routes,
            routes_with_api_calls,
            total_api_calls: reports.iter().map(|route| route.api_calls.len()).sum(),
            unique_api_calls: unique_api_calls.len(),
            duplicate_api_calls,
            dynamic_api_calls,
            cached_api_calls,
            client_api_calls,
            server_api_calls,
            rsc_api_calls,
        },
        routes: reports,
        duplicates,
        unsupported,
    };

    final_report.unsupported.sort_by(|a, b| {
        a.route
            .cmp(&b.route)
            .then(a.file.cmp(&b.file))
            .then(a.line.cmp(&b.line))
    });
    final_report
        .duplicates
        .sort_by(|a, b| a.key.cmp(&b.key).then(a.count.cmp(&b.count)));

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&final_report)?);
    } else {
        print_markdown_report(&final_report);
    }

    Ok(())
}

fn normalize_target_pattern(target: &str) -> Option<String> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    })
}

fn route_matches_target(route_pattern: &str, target_raw: &str) -> bool {
    let Some(normalized_target) = normalize_target_pattern(target_raw) else {
        return false;
    };

    if normalized_target == "/" {
        return route_pattern == "/";
    }

    if normalized_target.ends_with('/') {
        let prefix = format!("{}/", normalized_target.trim_end_matches('/'));
        return route_pattern.starts_with(&prefix);
    }

    route_pattern == normalized_target
}

fn analyze_file(
    path: &Path,
    root: &Path,
    visited: &mut HashSet<(PathBuf, bool, bool)>,
    fetches: &mut Vec<FetchOccurrence>,
    cache: &mut Cache,
    inherited_is_client: bool,
    inherited_is_route_handler: bool,
) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    let abs_path = path.canonicalize()?;
    let visit_key = (
        abs_path.clone(),
        inherited_is_client,
        inherited_is_route_handler,
    );
    if visited.contains(&visit_key) {
        return Ok(false);
    }
    visited.insert(visit_key);

    let cache_key = (
        abs_path.clone(),
        inherited_is_client,
        inherited_is_route_handler,
    );
    if let Some(cached_fetches) = cache.files.get(&cache_key) {
        fetches.extend(cached_fetches.fetches.clone());
        return Ok(cached_fetches.is_client);
    }

    let source = std::fs::read_to_string(&abs_path)?;
    let rel_file = relative_string(root, &abs_path);

    let mut file_fetches = Vec::new();
    let is_client = ast::with_program(path, &source, |program, source| -> Result<bool> {
        let is_client = inherited_is_client
            || (!inherited_is_route_handler
                && program
                    .directives
                    .iter()
                    .any(|d| d.directive == "use client"));
        let mut visitor = FetchVisitor {
            source,
            file: rel_file,
            fetches: Vec::new(),
            is_client,
            is_route_handler: inherited_is_route_handler,
            cached_function: None,
            cached_kind: None,
        };
        visitor.visit_program(program);
        file_fetches.extend(visitor.fetches);
        let imports = collect_imports_from_program(&abs_path, program, source, &mut cache.imports)?;
        for import in imports {
            analyze_file(
                &import,
                root,
                visited,
                &mut file_fetches,
                cache,
                is_client,
                inherited_is_route_handler,
            )?;
        }
        Ok(is_client)
    })??;

    let cached = CachedFile {
        is_client,
        fetches: file_fetches.clone(),
    };
    cache.files.insert(cache_key.clone(), cached.clone());
    if cached.is_client != inherited_is_client {
        cache.files.insert(
            (abs_path.clone(), is_client, inherited_is_route_handler),
            cached,
        );
    }
    fetches.extend(file_fetches);

    Ok(is_client)
}

fn collect_imports(
    path: &Path,
    import_cache: &mut HashMap<PathBuf, Vec<PathBuf>>,
) -> Result<Vec<PathBuf>> {
    let abs_path = path.canonicalize()?;
    if let Some(cached_imports) = import_cache.get(&abs_path) {
        return Ok(cached_imports.clone());
    }

    let source = std::fs::read_to_string(&abs_path)?;
    let mut imports = Vec::new();
    ast::with_program(path, &source, |program, source| -> Result<()> {
        imports = collect_imports_from_program(&abs_path, program, source, import_cache)?;
        Ok(())
    })??;
    Ok(imports)
}

fn collect_imports_from_program<'a>(
    abs_path: &Path,
    program: &oxc_ast::ast::Program<'a>,
    source: &str,
    import_cache: &mut HashMap<PathBuf, Vec<PathBuf>>,
) -> Result<Vec<PathBuf>> {
    if let Some(cached_imports) = import_cache.get(abs_path) {
        return Ok(cached_imports.clone());
    }

    let mut imports = Vec::new();
    for stmt in &program.body {
        match stmt {
            Statement::ImportDeclaration(import) if is_runtime_import(import) => {
                if let Some(resolved) = resolve_import(abs_path, import.source.value.as_str()) {
                    imports.push(resolved);
                }
            }
            Statement::ExportNamedDeclaration(export) => {
                if !is_runtime_export(export, source) {
                    continue;
                }
                if let Some(source) = &export.source {
                    if let Some(resolved) = resolve_import(abs_path, source.value.as_str()) {
                        imports.push(resolved);
                    }
                }
            }
            Statement::ExportAllDeclaration(export) => {
                if export.export_kind == ImportOrExportKind::Type {
                    continue;
                }
                if let Some(resolved) = resolve_import(abs_path, export.source.value.as_str()) {
                    imports.push(resolved);
                }
            }
            _ => {}
        }
    }

    import_cache.insert(abs_path.to_path_buf(), imports.clone());
    Ok(imports)
}

fn is_runtime_import(import: &oxc_ast::ast::ImportDeclaration) -> bool {
    if import.import_kind == ImportOrExportKind::Type {
        return false;
    }

    let Some(specifiers) = &import.specifiers else {
        return true;
    };
    if specifiers.is_empty() {
        return true;
    }

    for specifier in specifiers {
        match specifier {
            ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => return true,
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => return true,
            ImportDeclarationSpecifier::ImportSpecifier(import_specifier) => {
                if import_specifier.import_kind == ImportOrExportKind::Value {
                    return true;
                }
            }
        }
    }

    false
}

fn is_runtime_export(export: &ExportNamedDeclaration, source: &str) -> bool {
    let raw = declaration_text(
        export.span().start as usize,
        export.span().end as usize,
        source,
    )
    .trim_start();
    if raw.starts_with("export type ") {
        return false;
    }

    match parse_named_specifiers(raw) {
        Some(named_specifiers) => {
            if named_specifiers.is_empty() {
                return true;
            }
            named_specifiers
                .iter()
                .any(|specifier| !specifier.trim_start().starts_with("type "))
        }
        None => true,
    }
}

fn declaration_text(start: usize, end: usize, source: &str) -> &str {
    if start > end || end > source.len() {
        return "";
    }
    &source[start..end]
}

fn parse_named_specifiers(statement: &str) -> Option<Vec<&str>> {
    let start = statement.find('{')?;
    let end = statement.rfind('}')?;
    if end <= start {
        return Some(Vec::new());
    }
    let names = statement[start + 1..end]
        .split(',')
        .map(|segment| segment.trim())
        .filter(|segment| !segment.is_empty())
        .collect();
    Some(names)
}

fn is_route_handler_file(path: &Path) -> bool {
    path.file_stem().and_then(|stem| stem.to_str()) == Some("route")
}

fn route_reaches_target(
    path: &Path,
    target: &Path,
    visited: &mut HashSet<PathBuf>,
    import_cache: &mut HashMap<PathBuf, Vec<PathBuf>>,
) -> Result<bool> {
    let abs_path = path.canonicalize()?;
    if abs_path == target {
        return Ok(true);
    }
    if visited.contains(&abs_path) {
        return Ok(false);
    }
    visited.insert(abs_path.clone());

    for import in collect_imports(&abs_path, import_cache)? {
        if route_reaches_target(&import, target, visited, import_cache)? {
            return Ok(true);
        }
    }

    Ok(false)
}

fn collect_layout_chain_files(route_file: &Path, frontend_root: &Path) -> Vec<PathBuf> {
    let mut layout_files = Vec::new();
    let mut current = route_file.parent();
    while let Some(parent) = current {
        if !parent.starts_with(frontend_root) {
            break;
        }

        for stem in ["layout", "loading", "error", "not-found", "template"] {
            for ext in ["tsx", "ts", "jsx", "js"] {
                let layout_file = parent.join(format!("{stem}.{ext}"));
                if layout_file.exists() {
                    layout_files.push(layout_file);
                }
            }
        }

        current = parent.parent();
    }

    layout_files
}

fn cache_kind_name(cache_kind: &CacheKind) -> &'static str {
    match cache_kind {
        CacheKind::None => "none",
        CacheKind::FetchCache => "fetch-cache",
        CacheKind::FetchNextRevalidate => "next-revalidate",
        CacheKind::FetchNextTags => "next-tags",
        CacheKind::ReactCache => "react-cache",
        CacheKind::Cache => "cache",
        CacheKind::UnstableCache => "unstable-cache",
    }
}

fn fetch_cache_label(fetch: &FetchOccurrence) -> String {
    if !fetch.cached {
        return "no".to_string();
    }

    let kind = cache_kind_name(&fetch.cache_kind);
    match &fetch.cached_function {
        Some(cached_function) => format!("{kind} ({cached_function})"),
        None => kind.to_string(),
    }
}

fn resolve_target_file(root: &Path, target: &str) -> Result<PathBuf> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        anyhow::bail!("target path cannot be empty");
    }

    let candidate = if Path::new(trimmed).is_absolute() {
        Path::new(trimmed).to_path_buf()
    } else {
        root.join(trimmed)
    };
    if !candidate.is_file() {
        anyhow::bail!("target path is not a file: {}", candidate.display());
    }
    Ok(candidate.canonicalize()?)
}

#[cfg(test)]
fn is_client_route_file(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    let source = std::fs::read_to_string(path)?;
    ast::with_program(path, &source, |program, _| {
        program
            .directives
            .iter()
            .any(|directive| directive.directive == "use client")
    })
}

fn resolve_import(current_file: &Path, specifier: &str) -> Option<PathBuf> {
    const RUNTIME_EXTENSIONS: [&str; 4] = ["tsx", "ts", "jsx", "js"];

    if specifier.starts_with('.') {
        let parent = current_file.parent()?;
        let joined = parent.join(specifier);
        if joined.exists() && joined.is_file() {
            if !joined
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| RUNTIME_EXTENSIONS.contains(&ext))
            {
                return None;
            }
            return Some(joined);
        }
        for ext in RUNTIME_EXTENSIONS {
            let path = joined.with_extension(ext);
            if path.exists() {
                return Some(path);
            }
            let index = joined.join(format!("index.{ext}"));
            if index.exists() {
                return Some(index);
            }
        }
    }
    None
}

fn relative_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn print_markdown_report(report: &FinalReport) {
    println!("# Next.js Fetch API Analysis");
    println!();
    println!("## Summary");
    println!("- Total Routes: {}", report.summary.total_routes);
    println!(
        "- Routes with API Calls: {}",
        report.summary.routes_with_api_calls
    );
    println!("- Total API Calls: {}", report.summary.total_api_calls);
    println!("- Unique API Calls: {}", report.summary.unique_api_calls);
    println!(
        "- Duplicate API Calls: {}",
        report.summary.duplicate_api_calls
    );
    println!("- Dynamic API Calls: {}", report.summary.dynamic_api_calls);
    println!("- Cached API Calls: {}", report.summary.cached_api_calls);
    println!("- Server API Calls: {}", report.summary.server_api_calls);
    println!("- RSC API Calls: {}", report.summary.rsc_api_calls);
    println!("- Client API Calls: {}", report.summary.client_api_calls);
    println!();

    println!("## Routes");
    for route in &report.routes {
        println!("### {} ({})", route.route, route.file);
        if route.api_calls.is_empty() {
            println!("(no fetches found)");
        } else {
            println!("| Method | Path | Side | File | Line | RSC | Dynamic | Cache |");
            println!("| --- | --- | --- | --- | --- | --- | --- | --- |");
            let mut unique_fetches = route.api_calls.clone();
            unique_fetches.sort();
            unique_fetches.dedup();
            for fetch in &unique_fetches {
                println!(
                    "| {} | `{}` | {} | {} | {} | {} | {} | {} |",
                    fetch.method,
                    fetch.path,
                    if matches!(fetch.side, FetchSide::Client) {
                        "client"
                    } else {
                        "server"
                    },
                    fetch.file,
                    fetch.line,
                    if fetch.rsc { "yes" } else { "no" },
                    if fetch.dynamic { "✅" } else { "❌" },
                    fetch_cache_label(fetch)
                );
            }
        }
        println!();
    }

    if !report.duplicates.is_empty() {
        println!("## Duplicates");
        println!("| Key | Count | Route | File | Line |");
        println!("| --- | --- | --- | --- | --- |");
        for fetch in &report.duplicates {
            for occurrence in &fetch.occurrences {
                println!(
                    "| `{}` | {} | {} | {} | {} |",
                    fetch.key, fetch.count, occurrence.route, occurrence.file, occurrence.line
                );
            }
        }
        println!();
    }

    if !report.unsupported.is_empty() {
        println!("## Unsupported (Dynamic)");
        println!("| Route | File | Line | Reason | Path |");
        println!("| --- | --- | --- | --- | --- |");
        for fetch in &report.unsupported {
            println!(
                "| {} | {} | {} | {} | `{}` |",
                fetch.route, fetch.file, fetch.line, fetch.reason, fetch.raw_path
            );
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn first_call_expression<'a>(
        statement: &'a oxc_ast::ast::Statement<'a>,
    ) -> &'a oxc_ast::ast::CallExpression<'a> {
        let oxc_ast::ast::Statement::ExpressionStatement(expr_stmt) = statement else {
            panic!("expected expression statement");
        };
        let oxc_ast::ast::Expression::CallExpression(call) = &expr_stmt.expression else {
            panic!("expected call expression");
        };
        call
    }

    fn first_statement_assignment_call_expression<'a>(
        statement: &'a oxc_ast::ast::Statement<'a>,
    ) -> &'a oxc_ast::ast::CallExpression<'a> {
        let oxc_ast::ast::Statement::ExpressionStatement(expr_stmt) = statement else {
            panic!("expected expression statement");
        };
        let oxc_ast::ast::Expression::AssignmentExpression(assignment) = &expr_stmt.expression
        else {
            panic!("expected assignment expression");
        };
        let oxc_ast::ast::Expression::CallExpression(call) = &assignment.right else {
            panic!("expected cache wrapper call expression");
        };
        call
    }

    fn object_argument_from_call_expression<'a>(
        call: &'a oxc_ast::ast::CallExpression<'a>,
    ) -> &'a oxc_ast::ast::ObjectExpression<'a> {
        let Argument::ObjectExpression(obj) = &call.arguments[1] else {
            panic!("expected object argument");
        };
        obj
    }

    #[test]
    #[should_panic]
    fn test_first_call_expression_panics_when_not_expression_statement() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "if (true) {}";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        first_call_expression(&parsed.program.body[0]);
    }

    #[test]
    #[should_panic]
    fn test_first_call_expression_panics_when_not_call_expression() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "value;";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        first_call_expression(&parsed.program.body[0]);
    }

    #[test]
    #[should_panic]
    fn test_first_statement_assignment_call_expression_panics_when_not_expression_statement() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "if (true) {}";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        first_statement_assignment_call_expression(&parsed.program.body[0]);
    }

    #[test]
    #[should_panic]
    fn test_first_statement_assignment_call_expression_panics_when_not_assignment_expression() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "cache(() => {});";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        first_statement_assignment_call_expression(&parsed.program.body[0]);
    }

    #[test]
    #[should_panic]
    fn test_first_statement_assignment_call_expression_panics_when_right_not_call_expression() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "cachedFn = helper;";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        first_statement_assignment_call_expression(&parsed.program.body[0]);
    }

    #[test]
    #[should_panic]
    fn test_object_argument_from_call_expression_panics_when_not_object_argument() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "fetch('/api', '/not-object')";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let call = first_call_expression(&parsed.program.body[0]);
        object_argument_from_call_expression(call);
    }

    #[test]
    fn test_extract_string_literal_from_argument_none() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "fetch(true)";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let call = first_call_expression(&parsed.program.body[0]);
        let arg = &call.arguments[0];
        assert_eq!(
            extract_url_from_argument(arg, source),
            UrlExtraction {
                path: "dynamic".to_string(),
                raw_path: "true".to_string(),
                is_dynamic: true,
                is_unsupported: true,
            }
        );
    }

    #[test]
    fn test_extract_url_from_argument_works_for_direct_call_expression() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "fetch('/api/direct');";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let call = first_call_expression(&parsed.program.body[0]);
        let result = extract_url_from_argument(&call.arguments[0], source);
        assert_eq!(
            result,
            UrlExtraction {
                path: "/api/direct".to_string(),
                raw_path: "/api/direct".to_string(),
                is_dynamic: false,
                is_unsupported: false,
            }
        );
    }

    #[test]
    fn test_extract_url_from_argument_works_for_nonnumeric_argument() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "fetch(123)";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let call = first_call_expression(&parsed.program.body[0]);
        let arg = &call.arguments[0];
        assert_eq!(
            extract_url_from_argument(arg, source),
            UrlExtraction {
                path: "dynamic".to_string(),
                raw_path: "123".to_string(),
                is_dynamic: true,
                is_unsupported: true,
            }
        );
    }

    #[test]
    fn test_extract_url_from_argument_works_for_template_literal() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "fetch(`/api/foo`)";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let call = first_call_expression(&parsed.program.body[0]);
        let arg = &call.arguments[0];
        assert_eq!(
            extract_url_from_argument(arg, source),
            UrlExtraction {
                path: "/api/foo".to_string(),
                raw_path: "`/api/foo`".to_string(),
                is_dynamic: false,
                is_unsupported: false,
            }
        );
    }

    #[test]
    fn test_infer_cached_wrapper_name_parses_cached_identifiers() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "cachedFn = cache(() => {});";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let call = first_statement_assignment_call_expression(&parsed.program.body[0]);
        assert_eq!(
            infer_cached_wrapper_name(source, call),
            Some("cachedFn".to_string())
        );
    }

    #[test]
    fn test_infer_cached_wrapper_name_returns_none_for_direct_call() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "cache(() => {});";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let call = first_call_expression(&parsed.program.body[0]);
        assert_eq!(infer_cached_wrapper_name(source, call), None);
    }

    #[test]
    fn test_infer_cached_wrapper_name_returns_none_for_member_access_target() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "obj['value'] = cache(() => {});";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let call = first_statement_assignment_call_expression(&parsed.program.body[0]);
        assert_eq!(infer_cached_wrapper_name(source, call), None);
    }

    #[test]
    fn test_infer_cached_wrapper_name_ignores_non_ascii_target() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "µcached = cache(() => {});";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let call = first_statement_assignment_call_expression(&parsed.program.body[0]);
        assert_eq!(infer_cached_wrapper_name(source, call), None);
    }

    #[test]
    fn test_infer_cached_wrapper_name_returns_none_for_non_identifier_assignment_target() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "obj.cached_fn = cache(() => {});";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let call = first_statement_assignment_call_expression(&parsed.program.body[0]);
        assert_eq!(infer_cached_wrapper_name(source, call), None);
    }

    #[test]
    fn test_extract_fetch_cache_options_cache_non_string() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "fetch('/api/no-store', { cache: true });";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let call = first_call_expression(&parsed.program.body[0]);
        let obj = object_argument_from_call_expression(call);

        let (cached, kind) = extract_fetch_cache_options(obj);
        assert!(!cached);
        assert_eq!(kind, CacheKind::None);
    }

    #[test]
    fn test_extract_fetch_cache_options_next_unknown_property() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "fetch('/api/next', { next: { foo: true, revalidate: 0 } });";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let call = first_call_expression(&parsed.program.body[0]);
        let obj = object_argument_from_call_expression(call);

        let (cached, kind) = extract_fetch_cache_options(obj);
        assert!(!cached);
        assert_eq!(kind, CacheKind::None);
    }

    #[test]
    fn test_extract_fetch_cache_options_next_non_object() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "fetch('/api/next', { next: 60 });";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let call = first_call_expression(&parsed.program.body[0]);
        let obj = object_argument_from_call_expression(call);

        let (cached, kind) = extract_fetch_cache_options(obj);
        assert!(!cached);
        assert_eq!(kind, CacheKind::None);
    }

    #[test]
    fn test_extract_fetch_cache_options_next_computed_property_is_ignored() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "fetch('/api/next', { next: { [foo]: 60, revalidate: 60 } });";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let call = first_call_expression(&parsed.program.body[0]);
        let obj = object_argument_from_call_expression(call);

        let (cached, kind) = extract_fetch_cache_options(obj);
        assert!(cached);
        assert_eq!(kind, CacheKind::FetchNextRevalidate);
    }

    #[test]
    fn test_source_text_handles_invalid_slices() {
        assert!(source_text(1, 0, "abc").is_none());
        assert!(source_text(0, 4, "abc").is_none());
        assert!(source_text(1, 2, "é").is_none());
        assert_eq!(source_text(0, 2, "é"), Some("é".to_string()));
    }

    #[test]
    fn test_source_text_out_of_bounds_returns_empty_string_for_declaration_text() {
        assert_eq!(declaration_text(10, 5, "abc"), "");
        assert_eq!(declaration_text(0, 5, "abc"), "");
        assert_eq!(declaration_text(0, 2, "abc"), "ab");
    }

    #[test]
    fn test_parse_named_specifiers_returns_empty_when_invalid_order() {
        assert_eq!(parse_named_specifiers("}{"), Some(Vec::new()));
    }

    #[test]
    fn test_visitor_cache_options_unknown_flags_are_ignored() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "
            fetch('/api/no-store', { cache: 'no-store' });
            fetch('/api/not-object', { next: ['revalidate'] });
            fetch('/api/next-unknown', { next: { unknown: true, ...tags }});
        ";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let mut visitor = FetchVisitor {
            source,
            file: "test.ts".to_string(),
            fetches: Vec::new(),
            is_client: false,
            is_route_handler: false,
            cached_function: None,
            cached_kind: None,
        };
        visitor.visit_program(&parsed.program);
        assert_eq!(visitor.fetches.len(), 3);
        for fetch in &visitor.fetches {
            assert!(!fetch.cached);
        }

        assert_eq!(visitor.fetches[0].cache_kind, CacheKind::None);
        assert_eq!(visitor.fetches[1].cache_kind, CacheKind::None);
        assert_eq!(visitor.fetches[2].cache_kind, CacheKind::None);
    }

    #[test]
    fn test_extract_fetch_cache_options_force_cache() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "fetch('/api/cache', { cache: 'force-cache' });";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let call = first_call_expression(&parsed.program.body[0]);
        let obj = object_argument_from_call_expression(call);

        let (cached, kind) = extract_fetch_cache_options(obj);
        assert!(cached);
        assert_eq!(kind, CacheKind::FetchCache);
    }

    #[test]
    fn test_extract_fetch_cache_options_next_revalidate() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "fetch('/api/next', { next: { revalidate: 60 } });";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let call = first_call_expression(&parsed.program.body[0]);
        let obj = object_argument_from_call_expression(call);

        let (cached, kind) = extract_fetch_cache_options(obj);
        assert!(cached);
        assert_eq!(kind, CacheKind::FetchNextRevalidate);
    }

    #[test]
    fn test_extract_fetch_cache_options_tags() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "fetch('/api/tags', { next: { tags: ['alpha'] } });";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let call = first_call_expression(&parsed.program.body[0]);
        let obj = object_argument_from_call_expression(call);

        let (cached, kind) = extract_fetch_cache_options(obj);
        assert!(cached);
        assert_eq!(kind, CacheKind::FetchNextTags);
    }

    #[test]
    fn test_collect_imports_reuses_cached_imports() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("pkg")).unwrap();
        fs::write(dir.path().join("pkg/side-effect.ts"), "").unwrap();
        fs::write(dir.path().join("pkg/types.ts"), "").unwrap();
        let file = dir.path().join("pkg/index.ts");
        fs::write(
            &file,
            "
                import './side-effect';
                import type { Foo } from './types';
            ",
        )
        .unwrap();

        let mut import_cache = HashMap::new();
        let first = collect_imports(&file, &mut import_cache).unwrap();
        let second = collect_imports(&file, &mut import_cache).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn test_visitor_non_fetch() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "notFetch();";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let mut visitor = FetchVisitor {
            source,
            file: "test.ts".to_string(),
            fetches: Vec::new(),
            is_client: false,
            is_route_handler: false,
            cached_function: None,
            cached_kind: None,
        };
        visitor.visit_program(&parsed.program);
        assert_eq!(visitor.fetches.len(), 0);
    }

    #[test]
    fn test_visitor_complex_variants() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "
            fetch(url, options);
            fetch(url, { notMethod: 'POST' });
            fetch(url, { method: methodVar });
            fetch(url, { ...spread });
            fetch(url, { [dynamic]: 'POST' });
        ";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let mut visitor = FetchVisitor {
            source,
            file: "test.ts".to_string(),
            fetches: Vec::new(),
            is_client: false,
            is_route_handler: false,
            cached_function: None,
            cached_kind: None,
        };
        visitor.visit_program(&parsed.program);
        assert_eq!(visitor.fetches.len(), 5);
        for fetch in &visitor.fetches {
            assert_eq!(fetch.method, "GET");
            assert!(fetch.dynamic);
            assert!(fetch.line > 0);
        }
    }

    #[test]
    fn test_visitor_cache_options_are_extracted() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "
            fetch('/api/cache', { cache: 'force-cache' });
            fetch('/api/next', { next: { revalidate: 60 }});
            fetch('/api/next-zero', { next: { revalidate: 0 }});
            fetch('/api/tags', { next: { tags: ['a', 'b'] }});
        ";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let mut visitor = FetchVisitor {
            source,
            file: "test.ts".to_string(),
            fetches: Vec::new(),
            is_client: false,
            is_route_handler: false,
            cached_function: None,
            cached_kind: None,
        };
        visitor.visit_program(&parsed.program);
        assert_eq!(visitor.fetches.len(), 4);
        assert!(visitor.fetches[0].cached);
        assert_eq!(visitor.fetches[0].cache_kind, CacheKind::FetchCache);
        assert!(visitor.fetches[1].cached);
        assert_eq!(
            visitor.fetches[1].cache_kind,
            CacheKind::FetchNextRevalidate
        );
        assert!(!visitor.fetches[2].cached);
        assert_eq!(visitor.fetches[2].cache_kind, CacheKind::None);
        assert!(visitor.fetches[3].cached);
        assert_eq!(visitor.fetches[3].cache_kind, CacheKind::FetchNextTags);
    }

    #[test]
    fn test_route_matches_target() {
        assert!(route_matches_target("/users", "users"));
        assert!(!route_matches_target("/users-team", "users"));
        assert!(route_matches_target("/users/team", "users/"));
        assert!(!route_matches_target("/users-team/page", "users/"));
        assert!(route_matches_target("/users", "/users"));
        assert!(!route_matches_target("/users/team", "/users"));
        assert!(route_matches_target("/users/team", "/users/"));
        assert!(route_matches_target("/", "/"));
        assert!(!route_matches_target("/users", "/"));
    }

    #[test]
    fn test_route_matches_target_rejects_empty_input() {
        assert!(!route_matches_target("/users", ""));
        assert!(!route_matches_target("/users", "   "));
    }

    #[test]
    fn test_visitor_cache_wrappers_mark_fetch_calls() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "
            cache(fetch('/api/cached', { method: 'POST' }));
            unstable_cache(fetch('/api/unstable', { next: { revalidate: 60 } }));
            const getUsers = cache(fetch('/api/users', { method: 'PUT' }));
        ";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let mut visitor = FetchVisitor {
            source,
            file: "test.ts".to_string(),
            fetches: Vec::new(),
            is_client: false,
            is_route_handler: false,
            cached_function: None,
            cached_kind: None,
        };
        visitor.visit_program(&parsed.program);
        assert_eq!(visitor.fetches.len(), 3);

        assert!(visitor.fetches[0].cached);
        assert_eq!(visitor.fetches[0].cache_kind, CacheKind::ReactCache);
        assert_eq!(visitor.fetches[0].cached_function.as_deref(), Some("cache"));
        assert_eq!(visitor.fetches[0].method, "POST");

        assert!(visitor.fetches[1].cached);
        assert_eq!(visitor.fetches[1].cache_kind, CacheKind::UnstableCache);
        assert_eq!(
            visitor.fetches[1].cached_function.as_deref(),
            Some("unstable_cache")
        );
        assert_eq!(visitor.fetches[1].method, "GET");

        assert!(visitor.fetches[2].cached);
        assert_eq!(visitor.fetches[2].cache_kind, CacheKind::ReactCache);
        assert_eq!(
            visitor.fetches[2].cached_function.as_deref(),
            Some("getUsers")
        );
        assert_eq!(visitor.fetches[2].method, "PUT");
    }

    #[test]
    fn test_visitor_no_args() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "fetch();";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let mut visitor = FetchVisitor {
            source,
            file: "test.ts".to_string(),
            fetches: Vec::new(),
            is_client: false,
            is_route_handler: false,
            cached_function: None,
            cached_kind: None,
        };
        visitor.visit_program(&parsed.program);
        assert_eq!(visitor.fetches.len(), 1);
        assert_eq!(visitor.fetches[0].path, "unknown");
        assert!(visitor.fetches[0].dynamic);
        assert!(visitor.fetches[0].unsupported);
    }

    #[test]
    fn test_visitor_dynamic_and_template() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "fetch(url); fetch(`/api/${id}`, { method: 'PATCH' }); fetch('/api/get');";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let mut visitor = FetchVisitor {
            source,
            file: "test.ts".to_string(),
            fetches: Vec::new(),
            is_client: false,
            is_route_handler: false,
            cached_function: None,
            cached_kind: None,
        };
        visitor.visit_program(&parsed.program);
        assert_eq!(visitor.fetches.len(), 3);
        assert_eq!(visitor.fetches[0].path, "dynamic");
        assert!(visitor.fetches[0].dynamic);
        assert!(visitor.fetches[0].rsc);
        assert_eq!(visitor.fetches[1].path, "/api/${id}");
        assert!(visitor.fetches[1].dynamic);
        assert_eq!(visitor.fetches[1].method, "PATCH");
        assert!(visitor.fetches[1].rsc);
        assert_eq!(visitor.fetches[2].path, "/api/get");
        assert!(!visitor.fetches[2].dynamic);
        assert_eq!(visitor.fetches[2].method, "GET");
        assert!(visitor.fetches[2].rsc);
    }

    #[test]
    fn test_visitor_route_handler_fetches_are_non_rsc() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "fetch('/api/route');";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let mut visitor = FetchVisitor {
            source,
            file: "app/api/route.ts".to_string(),
            fetches: Vec::new(),
            is_client: false,
            is_route_handler: true,
            cached_function: None,
            cached_kind: None,
        };
        visitor.visit_program(&parsed.program);
        assert_eq!(visitor.fetches.len(), 1);
        assert!(!visitor.fetches[0].rsc);
    }

    #[test]
    fn test_route_report_api_calls_uses_camel_case() {
        let report = RouteReport {
            route: "/".to_string(),
            file: "app/page.tsx".to_string(),
            api_calls: vec![FetchOccurrence {
                path: "/api/example".to_string(),
                raw_path: "/api/example".to_string(),
                method: "GET".to_string(),
                file: "app/page.tsx".to_string(),
                line: 3,
                side: FetchSide::Server,
                rsc: true,
                cached: false,
                cache_kind: CacheKind::None,
                cached_function: None,
                dynamic: false,
                unsupported: false,
            }],
        };
        let serialized = serde_json::to_string(&report).unwrap();
        assert!(serialized.contains("\"apiCalls\""));
        assert!(!serialized.contains("\"api_calls\""));
    }

    #[test]
    fn test_resolve_import_index() {
        let dir = tempdir().unwrap();
        let lib = dir.path().join("lib");
        fs::create_dir(&lib).unwrap();
        fs::write(lib.join("index.ts"), "").unwrap();

        let current = dir.path().join("main.ts");
        let resolved = resolve_import(&current, "./lib").unwrap();
        assert!(resolved.ends_with("lib/index.ts"));
    }

    #[test]
    fn test_resolve_import_explicit_extension() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("lib.ts");
        fs::write(&file, "").unwrap();

        let current = dir.path().join("main.ts");
        let resolved = resolve_import(&current, "./lib.ts").unwrap();
        assert_eq!(
            resolved.canonicalize().unwrap(),
            file.canonicalize().unwrap()
        );
    }

    #[test]
    fn test_resolve_import_skips_non_javascript_file() {
        let dir = tempdir().unwrap();
        let stylesheet = dir.path().join("styles.css");
        fs::write(&stylesheet, "body { }").unwrap();

        let current = dir.path().join("main.ts");
        assert_eq!(resolve_import(&current, "./styles"), None);
    }

    #[test]
    fn test_resolve_import_skips_existing_non_javascript_file_without_extension() {
        let dir = tempdir().unwrap();
        let non_js = dir.path().join("legacy");
        fs::write(&non_js, "legacy").unwrap();
        let current = dir.path().join("main.ts");

        assert_eq!(resolve_import(&current, "./legacy"), None);
    }

    #[test]
    fn test_resolve_import_root() {
        assert_eq!(resolve_import(Path::new("page.ts"), "./lib"), None);
    }

    #[test]
    fn test_resolve_import_non_dot_specifier() {
        let dir = tempdir().unwrap();
        let current = dir.path().join("main.ts");

        assert_eq!(resolve_import(&current, "react"), None);
    }

    #[test]
    fn test_fetch_cache_kind_names() {
        assert_eq!(cache_kind_name(&CacheKind::None), "none");
        assert_eq!(cache_kind_name(&CacheKind::FetchCache), "fetch-cache");
        assert_eq!(
            cache_kind_name(&CacheKind::FetchNextRevalidate),
            "next-revalidate"
        );
        assert_eq!(cache_kind_name(&CacheKind::FetchNextTags), "next-tags");
        assert_eq!(cache_kind_name(&CacheKind::ReactCache), "react-cache");
        assert_eq!(cache_kind_name(&CacheKind::Cache), "cache");
        assert_eq!(cache_kind_name(&CacheKind::UnstableCache), "unstable-cache");
    }

    #[test]
    fn test_fetch_cache_label_includes_cached_function() {
        let fetch = FetchOccurrence {
            path: "/api/example".to_string(),
            raw_path: "/api/example".to_string(),
            method: "GET".to_string(),
            file: "app/page.tsx".to_string(),
            line: 1,
            side: FetchSide::Server,
            rsc: true,
            cached: true,
            cache_kind: CacheKind::ReactCache,
            cached_function: Some("cache".to_string()),
            dynamic: false,
            unsupported: false,
        };
        assert_eq!(fetch_cache_label(&fetch), "react-cache (cache)");
    }

    #[test]
    fn test_fetch_cache_label_without_cached_function() {
        let fetch = FetchOccurrence {
            path: "/api/example".to_string(),
            raw_path: "/api/example".to_string(),
            method: "GET".to_string(),
            file: "app/page.tsx".to_string(),
            line: 1,
            side: FetchSide::Server,
            rsc: true,
            cached: true,
            cache_kind: CacheKind::FetchCache,
            cached_function: None,
            dynamic: false,
            unsupported: false,
        };
        assert_eq!(fetch_cache_label(&fetch), "fetch-cache");
    }

    #[test]
    fn test_is_route_handler_file_variants() {
        assert!(is_route_handler_file(Path::new("route.ts")));
        assert!(is_route_handler_file(Path::new("route")));
        assert!(!is_route_handler_file(Path::new("page.tsx")));
        assert!(!is_route_handler_file(Path::new("not-route.txt")));
    }

    #[test]
    fn test_collect_layout_chain_files_includes_parent_chain() {
        let dir = tempdir().unwrap();
        let app = dir.path().join("app");
        fs::create_dir_all(app.join("dashboard")).unwrap();

        fs::write(app.join("layout.tsx"), "export {}").unwrap();
        fs::write(app.join("template.tsx"), "export {}").unwrap();
        fs::write(app.join("dashboard/layout.tsx"), "export {}").unwrap();
        fs::write(app.join("dashboard/template.tsx"), "export {}").unwrap();
        let page = app.join("dashboard/page.tsx");
        fs::write(&page, "export {}").unwrap();

        let chain = collect_layout_chain_files(&page, &app);
        assert_eq!(chain.len(), 4);
        assert!(chain.contains(&app.join("template.tsx")));
        assert!(chain.contains(&app.join("dashboard/layout.tsx")));
        assert!(chain.contains(&app.join("layout.tsx")));
        assert!(chain.contains(&app.join("dashboard/template.tsx")));
    }

    #[test]
    fn test_is_client_route_file_missing_file() {
        assert!(!is_client_route_file(Path::new("does-not-exist.ts")).unwrap());
    }

    #[test]
    fn test_is_client_route_file_with_use_client_directive() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("client.ts");
        fs::write(&file, "'use client';\nexport {};").unwrap();

        assert!(is_client_route_file(&file).unwrap());
    }

    #[test]
    fn test_is_client_route_file_without_use_client_directive() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("server.ts");
        fs::write(&file, "export {};").unwrap();

        assert!(!is_client_route_file(&file).unwrap());
    }

    #[test]
    fn test_resolve_target_file_errors() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("absolute.ts");
        fs::write(&file, "").unwrap();

        let empty = resolve_target_file(dir.path(), "   ");
        assert!(empty.is_err());
        let err = empty.unwrap_err();
        assert!(err.to_string().contains("target path cannot be empty"));

        let absolute = file.canonicalize().unwrap();
        let resolved = resolve_target_file(dir.path(), absolute.to_str().unwrap()).unwrap();
        assert_eq!(resolved, absolute);

        let dir_target = dir.path().join("dir");
        fs::create_dir(&dir_target).unwrap();
        let not_file = resolve_target_file(dir.path(), "dir");
        assert!(not_file.is_err());
        let err = not_file.unwrap_err();
        assert!(err.to_string().contains("target path is not a file"));
    }

    #[test]
    fn test_route_reaches_target_short_circuit() {
        let dir = tempdir().unwrap();
        let route = dir.path().join("route.ts");
        let target = dir.path().join("target.ts");
        fs::write(&route, "").unwrap();
        fs::write(&target, "").unwrap();

        let mut cache = HashMap::new();
        let mut visited = HashSet::new();
        let route_abs = route.canonicalize().unwrap();
        let target_abs = target.canonicalize().unwrap();
        assert!(!route_reaches_target(&route, &target_abs, &mut visited, &mut cache).unwrap());

        visited.insert(route_abs);
        let matched_direct =
            route_reaches_target(&route, &target_abs, &mut visited, &mut cache).unwrap();
        assert!(!matched_direct);
    }

    #[test]
    fn test_route_reaches_target_matches_direct() {
        let dir = tempdir().unwrap();
        let route = dir.path().join("route.ts");
        fs::write(&route, "").unwrap();

        let mut cache = HashMap::new();
        let mut visited = HashSet::new();
        let route_abs = route.canonicalize().unwrap();
        let reached = route_reaches_target(&route, &route_abs, &mut visited, &mut cache).unwrap();
        assert!(reached);
    }

    #[test]
    fn test_route_reaches_target_via_import() {
        let dir = tempdir().unwrap();
        let route = dir.path().join("route.ts");
        let middle = dir.path().join("middle.ts");
        let target = dir.path().join("target.ts");
        fs::write(&route, "import { helper } from './middle';").unwrap();
        fs::write(&middle, "import { target } from './target';").unwrap();
        fs::write(&target, "").unwrap();

        let mut cache = HashMap::new();
        let mut visited = HashSet::new();
        assert!(route_reaches_target(
            &route,
            &target.canonicalize().unwrap(),
            &mut visited,
            &mut cache
        )
        .unwrap());
    }

    #[test]
    fn test_route_reaches_target_with_unmatched_import_chain() {
        let dir = tempdir().unwrap();
        let route = dir.path().join("route.ts");
        let middle = dir.path().join("middle.ts");
        let target = dir.path().join("target.ts");
        let leaf = dir.path().join("leaf.ts");
        fs::write(&route, "import { helper } from './middle';").unwrap();
        fs::write(&middle, "import { helper2 } from './leaf';").unwrap();
        fs::write(&leaf, "").unwrap();
        fs::write(&target, "").unwrap();

        let mut cache = HashMap::new();
        let mut visited = HashSet::new();
        assert!(!route_reaches_target(
            &route,
            &target.canonicalize().unwrap(),
            &mut visited,
            &mut cache
        )
        .unwrap());
    }

    #[test]
    fn test_is_runtime_import_variants() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "
            const constant = 1;
            import type { Foo } from './foo';
            import {} from './empty';
            import { type Bar } from './bar';
            import { Baz } from './baz';
            import Widget, { type Props } from './widget';
            import * as all from './all';
        ";
        let source_type = oxc_span::SourceType::from_path(std::path::Path::new("test.ts")).unwrap();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        assert!(
            parsed.errors.is_empty(),
            "import parser errors: {:?}",
            parsed.errors
        );
        let imports = parsed
            .program
            .body
            .iter()
            .filter_map(|stmt| match stmt {
                Statement::ImportDeclaration(import) => Some(import),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(imports.len(), 6);
        assert!(!is_runtime_import(imports[0]));
        assert!(is_runtime_import(imports[1]));
        assert!(!is_runtime_import(imports[2]));
        assert!(is_runtime_import(imports[3]));
        assert!(is_runtime_import(imports[4]));
        assert!(is_runtime_import(imports[5]));
    }

    #[test]
    fn test_is_runtime_export_variants() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "
            const constant = 1;
            export type { Foo } from './foo';
            export {};
            export { type Bar } from './bar';
            export { Baz } from './baz';
        ";
        let source_type = oxc_span::SourceType::from_path(std::path::Path::new("test.ts")).unwrap();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        assert!(
            parsed.errors.is_empty(),
            "export parser errors: {:?}",
            parsed.errors
        );
        let exports = parsed
            .program
            .body
            .iter()
            .filter_map(|stmt| match stmt {
                Statement::ExportNamedDeclaration(export) => Some(export),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(exports.len(), 4);
        assert!(!is_runtime_export(exports[0], source));
        assert!(is_runtime_export(exports[1], source));
        assert!(!is_runtime_export(exports[2], source));
        assert!(is_runtime_export(exports[3], source));
    }

    #[test]
    fn test_collect_imports_filters_runtime_and_type_only_imports_exports() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("pkg")).unwrap();

        fs::write(dir.path().join("pkg/side-effect.ts"), "").unwrap();
        fs::write(dir.path().join("pkg/runtime.ts"), "").unwrap();
        fs::write(dir.path().join("pkg/runtime-all.ts"), "").unwrap();
        fs::write(dir.path().join("pkg/types.ts"), "").unwrap();

        let file = dir.path().join("pkg/index.ts");
        fs::write(
            &file,
            "
            import './side-effect';
            import type { Foo } from './types';
            export type { Foo } from './types';
            export type * from './types';
            export { runtimeExport } from './runtime';
            export * from './runtime-all';
            ",
        )
        .unwrap();

        let mut import_cache = HashMap::new();
        let imports = collect_imports(&file, &mut import_cache).unwrap();
        assert_eq!(imports.len(), 3);
        assert!(imports.iter().any(|path| path.ends_with("side-effect.ts")));
        assert!(imports.iter().any(|path| path.ends_with("runtime.ts")));
        assert!(imports.iter().any(|path| path.ends_with("runtime-all.ts")));
        assert!(!imports.iter().any(|path| path.ends_with("types.ts")));
    }

    #[test]
    fn test_route_reaches_target_client_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("client.ts");
        fs::write(
            &file,
            "
            'use client';
            import { helper } from './helper';
            export {};
            ",
        )
        .unwrap();

        let helper = dir.path().join("helper.ts");
        fs::write(&helper, "export const helper = () => fetch('/api/helper');").unwrap();

        let mut cache = Cache {
            files: HashMap::new(),
            imports: HashMap::new(),
        };
        let mut visited = HashSet::new();
        let mut fetches = Vec::new();
        analyze_file(
            &file,
            dir.path(),
            &mut visited,
            &mut fetches,
            &mut cache,
            false,
            false,
        )
        .unwrap();
        assert!(fetches.iter().any(|fetch| fetch.path == "/api/helper"));
        assert!(fetches.iter().any(|fetch| !fetch.rsc));
    }

    #[test]
    fn test_resolve_import_none() {
        let dir = tempdir().unwrap();
        let current = dir.path().join("main.ts");
        assert_eq!(resolve_import(&current, "./missing"), None);
    }

    #[test]
    fn test_analyze_file_cache_hit() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("file.ts");
        fs::write(&file, "fetch('/api/cache')").unwrap();

        let mut cache = Cache {
            files: HashMap::new(),
            imports: HashMap::new(),
        };
        let mut visited = HashSet::new();
        let mut fetches = Vec::new();
        analyze_file(
            &file,
            dir.path(),
            &mut visited,
            &mut fetches,
            &mut cache,
            false,
            false,
        )
        .unwrap();
        assert_eq!(fetches.len(), 1);

        let mut visited2 = HashSet::new();
        let mut fetches2 = Vec::new();
        analyze_file(
            &file,
            dir.path(),
            &mut visited2,
            &mut fetches2,
            &mut cache,
            false,
            false,
        )
        .unwrap();
        assert_eq!(fetches2.len(), 1);
        assert_eq!(fetches2[0].path, "/api/cache");
    }

    #[test]
    fn test_analyze_file_cache_hit_reuses_client_state() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("file.ts");
        fs::write(&file, "'use client'; fetch('/api/cache')").unwrap();

        let mut cache = Cache {
            files: HashMap::new(),
            imports: HashMap::new(),
        };
        let mut visited = HashSet::new();
        let mut fetches = Vec::new();
        let route_is_client = analyze_file(
            &file,
            dir.path(),
            &mut visited,
            &mut fetches,
            &mut cache,
            false,
            false,
        )
        .unwrap();
        assert!(route_is_client);
        assert_eq!(cache.files.len(), 2);

        let mut visited2 = HashSet::new();
        let mut fetches2 = Vec::new();
        let route_is_client = analyze_file(
            &file,
            dir.path(),
            &mut visited2,
            &mut fetches2,
            &mut cache,
            false,
            false,
        )
        .unwrap();
        assert!(route_is_client);
        assert_eq!(cache.files.len(), 2);
        assert_eq!(fetches2.len(), 1);
        assert_eq!(fetches2[0].path, "/api/cache");
    }

    #[test]
    fn test_analyze_file_cache_hit_reuses_client_state_with_client_flag() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("file.ts");
        fs::write(&file, "'use client'; fetch('/api/cache')").unwrap();

        let mut cache = Cache {
            files: HashMap::new(),
            imports: HashMap::new(),
        };
        let mut visited = HashSet::new();
        let mut fetches = Vec::new();
        analyze_file(
            &file,
            dir.path(),
            &mut visited,
            &mut fetches,
            &mut cache,
            false,
            false,
        )
        .unwrap();
        assert_eq!(cache.files.len(), 2);

        let mut visited2 = HashSet::new();
        let mut fetches2 = Vec::new();
        let route_is_client = analyze_file(
            &file,
            dir.path(),
            &mut visited2,
            &mut fetches2,
            &mut cache,
            true,
            false,
        )
        .unwrap();
        assert!(route_is_client);
        assert_eq!(cache.files.len(), 2);
        assert_eq!(fetches2.len(), 1);
        assert_eq!(fetches2[0].path, "/api/cache");
    }

    #[test]
    fn test_analyze_file_imported_file_is_analyzed() {
        let dir = tempdir().unwrap();
        let helper = dir.path().join("helper.ts");
        fs::write(&helper, "export const helper = () => fetch('/api/helper');").unwrap();
        let file = dir.path().join("file.ts");
        fs::write(&file, "import { helper } from './helper';").unwrap();

        let mut cache = Cache {
            files: HashMap::new(),
            imports: HashMap::new(),
        };
        let mut visited = HashSet::new();
        let mut fetches = Vec::new();
        analyze_file(
            &file,
            dir.path(),
            &mut visited,
            &mut fetches,
            &mut cache,
            false,
            false,
        )
        .unwrap();

        assert_eq!(fetches.len(), 1);
        assert_eq!(fetches[0].path, "/api/helper");
    }

    #[test]
    fn test_analyze_file_not_exists() {
        let mut visited = HashSet::new();
        let mut fetches = Vec::new();
        let mut cache = Cache {
            files: HashMap::new(),
            imports: HashMap::new(),
        };
        analyze_file(
            Path::new("missing.ts"),
            Path::new("."),
            &mut visited,
            &mut fetches,
            &mut cache,
            false,
            false,
        )
        .unwrap();
        assert!(fetches.is_empty());
    }

    #[test]
    fn test_cli_no_fetches() {
        use assert_cmd::Command;

        let root = tempdir().unwrap();
        fs::create_dir(root.path().join("app")).unwrap();
        fs::write(
            root.path().join("app/page.tsx"),
            "export default function Page() { return null; }",
        )
        .unwrap();

        let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
        cmd.arg("--root").arg(root.path());
        cmd.assert()
            .success()
            .stdout(predicates::str::contains("(no fetches found)"));
    }

    #[test]
    fn test_cli_matches_explicit_target_file() {
        use assert_cmd::Command;

        let root = tempdir().unwrap();
        fs::create_dir(root.path().join("app")).unwrap();
        let page = root.path().join("app/page.tsx");
        fs::write(&page, "fetch('/api/explicit-target');").unwrap();

        let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
        cmd.arg("--root").arg(root.path()).arg(&page);
        cmd.assert()
            .success()
            .stdout(predicates::str::contains("/api/explicit-target"));
    }

    #[test]
    fn test_cli_target_file_match_uses_import_chain() {
        use assert_cmd::Command;

        let root = tempdir().unwrap();
        fs::create_dir(root.path().join("app")).unwrap();

        let page = root.path().join("app/page.tsx");
        let middle = root.path().join("app/middle.ts");
        let target = root.path().join("app/target.ts");
        fs::write(&page, "import { helper } from './middle';").unwrap();
        fs::write(&middle, "import { helper } from './target';").unwrap();
        fs::write(&target, "fetch('/api/targeted');").unwrap();

        let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
        cmd.arg("--root").arg(root.path()).arg(&target);
        cmd.assert()
            .success()
            .stdout(predicates::str::contains("/api/targeted"));
    }

    #[test]
    fn test_cli_target_matching_uses_layout_wrapper_chain() {
        use assert_cmd::Command;

        let root = tempdir().unwrap();
        let app = root.path().join("app");
        fs::create_dir_all(app.join("dashboard")).unwrap();

        let layout = app.join("layout.tsx");
        fs::write(&layout, "fetch('/api/layout');").unwrap();
        let page = app.join("dashboard/page.tsx");
        fs::write(&page, "fetch('/api/page');").unwrap();

        let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
        cmd.arg("--root").arg(root.path()).arg(&page);
        cmd.assert()
            .success()
            .stdout(predicates::str::contains("/api/layout"));
    }

    #[test]
    fn test_cli_target_file_match_uses_layout_import_chain() {
        use assert_cmd::Command;

        let root = tempdir().unwrap();
        let app = root.path().join("app");
        fs::create_dir_all(app.join("dashboard")).unwrap();

        let layout = app.join("dashboard/layout.tsx");
        fs::write(
            &layout,
            "
            import { helper } from './target';
            ",
        )
        .unwrap();
        let page = app.join("dashboard/page.tsx");
        fs::write(&page, "fetch('/api/page');").unwrap();
        let target = app.join("dashboard/target.ts");
        fs::write(&target, "export const helper = () => fetch('/api/target');").unwrap();

        let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
        cmd.arg("--root").arg(root.path()).arg(&target);
        cmd.assert()
            .success()
            .stdout(predicates::str::contains("/api/page"));
    }

    #[test]
    fn test_cli_includes_page_and_layout_routes_by_default() {
        use assert_cmd::Command;

        let root = tempdir().unwrap();
        fs::create_dir(root.path().join("app")).unwrap();
        fs::write(root.path().join("app/layout.tsx"), "fetch('/api/layout');").unwrap();
        fs::write(root.path().join("app/page.tsx"), "fetch('/api/page');").unwrap();

        let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
        cmd.arg("--root")
            .arg(root.path())
            .assert()
            .success()
            .stdout(predicates::str::contains("/api/page"))
            .stdout(predicates::str::contains("/api/layout"));
    }

    #[test]
    fn test_cli_includes_client_side_cached_duplicates() {
        use assert_cmd::Command;

        let root = tempdir().unwrap();
        fs::create_dir(root.path().join("app")).unwrap();
        fs::write(
            root.path().join("app/page.tsx"),
            "
            'use client';
            fetch('/api/cached', { cache: 'force-cache' });
            fetch('/api/cached', { cache: 'force-cache' });
            ",
        )
        .unwrap();

        let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
        cmd.arg("--root").arg(root.path());
        cmd.assert()
            .success()
            .stdout(predicates::str::contains("Cached API Calls: 2"))
            .stdout(predicates::str::contains("## Duplicates"));
    }

    #[test]
    fn test_cli_follows_imports_when_analyzing_routes() {
        use assert_cmd::Command;

        let root = tempdir().unwrap();
        fs::create_dir(root.path().join("app")).unwrap();
        fs::write(
            root.path().join("app/page.tsx"),
            "import { fetchUsers } from './helper';",
        )
        .unwrap();
        fs::write(
            root.path().join("app/helper.ts"),
            "export const fetchUsers = () => fetch('/api/users');",
        )
        .unwrap();

        let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
        cmd.arg("--root").arg(root.path());
        cmd.assert()
            .success()
            .stdout(predicates::str::contains("/api/users"));
    }

    #[test]
    fn test_cli_target_file_match_uses_wrapper_chain() {
        use assert_cmd::Command;

        let root = tempdir().unwrap();
        let app = root.path().join("app");
        fs::create_dir_all(app.join("dashboard")).unwrap();

        let layout = app.join("layout.tsx");
        fs::write(&layout, "fetch('/api/layout');").unwrap();
        let page = app.join("dashboard/page.tsx");
        fs::write(&page, "fetch('/api/page');").unwrap();

        let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
        cmd.arg("--root").arg(root.path()).arg(&layout);
        cmd.assert()
            .success()
            .stdout(predicates::str::contains("/api/layout"));
    }

    #[test]
    fn test_cli_target_file_match_uses_wrapper_chain_via_import() {
        use assert_cmd::Command;

        let root = tempdir().unwrap();
        let app = root.path().join("app");
        fs::create_dir_all(app.join("dashboard")).unwrap();

        let layout = app.join("dashboard/layout.tsx");
        fs::write(
            &layout,
            "
            import { target } from './target.ts';
            fetch('/api/layout');
            ",
        )
        .unwrap();
        let page = app.join("dashboard/page.tsx");
        fs::write(&page, "fetch('/api/page');").unwrap();
        let target = app.join("dashboard/target.ts");
        fs::write(&target, "export const target = 123;").unwrap();

        let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
        cmd.arg("--root").arg(root.path()).arg(&target);
        cmd.assert()
            .success()
            .stdout(predicates::str::contains("/api/layout"));
    }

    #[test]
    fn test_cli_includes_client_side_fetches() {
        use assert_cmd::Command;

        let root = tempdir().unwrap();
        fs::create_dir(root.path().join("app")).unwrap();
        fs::write(
            root.path().join("app/page.tsx"),
            "'use client';\nfetch('/api/client');",
        )
        .unwrap();

        let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
        cmd.arg("--root").arg(root.path());
        cmd.assert().success().stdout(predicates::str::contains(
            "| GET | `/api/client` | client |",
        ));
    }

    #[test]
    fn test_cli_sorts_multiple_unsupported_fetches() {
        use assert_cmd::Command;

        let root = tempdir().unwrap();
        fs::create_dir_all(root.path().join("app/about")).unwrap();
        fs::write(root.path().join("app/page.tsx"), "fetch(url);").unwrap();
        fs::write(root.path().join("app/about/page.tsx"), "fetch(dynamic);").unwrap();

        let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
        cmd.arg("--root").arg(root.path());
        cmd.assert()
            .success()
            .stdout(predicates::str::contains("## Unsupported (Dynamic)"))
            .stdout(predicates::str::contains("### / (app/page.tsx)"))
            .stdout(predicates::str::contains("### /about (app/about/page.tsx)"));
    }

    #[test]
    fn test_cli_includes_duplicates_and_unsupported_sections() {
        use assert_cmd::Command;

        let root = tempdir().unwrap();
        fs::create_dir(root.path().join("app")).unwrap();
        fs::write(
            root.path().join("app/page.tsx"),
            "
            fetch(`/api/${dynamic}`);
            fetch('/api/duplicate');
            fetch('/api/duplicate');
            ",
        )
        .unwrap();

        let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
        cmd.arg("--root").arg(root.path());
        cmd.assert()
            .success()
            .stdout(predicates::str::contains("## Duplicates"))
            .stdout(predicates::str::contains("## Unsupported (Dynamic)"));
    }

    #[test]
    fn test_cli_target_missing_reports_unmatched_error() {
        use assert_cmd::Command;

        let root = tempdir().unwrap();
        fs::create_dir(root.path().join("app")).unwrap();
        fs::write(root.path().join("app/page.tsx"), "fetch('/api/page');").unwrap();

        let mut cmd = Command::cargo_bin("next-to-fetch").unwrap();
        cmd.arg("--root").arg(root.path()).arg("does-not-exist.ts");
        cmd.assert()
            .code(2)
            .stderr(predicates::str::contains("Error: targets not found"));
    }

    #[test]
    fn test_relative_string_failure() {
        let root = Path::new("/root/a");
        let path = Path::new("/root/b");
        assert_eq!(relative_string(root, path), "/root/b");
    }

    #[test]
    fn test_analyze_file_already_visited() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("file.ts");
        fs::write(&file, "").unwrap();

        let mut visited = HashSet::new();
        visited.insert((file.canonicalize().unwrap(), false, false));
        let mut fetches = Vec::new();
        let mut cache = Cache {
            files: HashMap::new(),
            imports: HashMap::new(),
        };
        analyze_file(
            &file,
            dir.path(),
            &mut visited,
            &mut fetches,
            &mut cache,
            false,
            false,
        )
        .unwrap();
        assert!(fetches.is_empty());
    }

    #[test]
    fn test_analyze_file_read_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("dir.ts");
        fs::create_dir(&path).unwrap();
        let mut visited = HashSet::new();
        let mut fetches = Vec::new();
        let mut cache = Cache {
            files: HashMap::new(),
            imports: HashMap::new(),
        };
        let err = analyze_file(
            &path,
            dir.path(),
            &mut visited,
            &mut fetches,
            &mut cache,
            false,
            false,
        )
        .err()
        .unwrap();
        assert!(
            err.to_string().contains("failed to read")
                || err.to_string().contains("Is a directory")
        );
    }
}
