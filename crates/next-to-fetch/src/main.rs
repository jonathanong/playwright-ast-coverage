use anyhow::Result;
use clap::Parser;
use no_mistakes_core::ast;
use no_mistakes_core::config;
use no_mistakes_core::routes;
use oxc_ast::ast::{Argument, CallExpression, Expression};
use oxc_ast_visit::{walk, Visit};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

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
    url: String,
    method: String,
    file: String,
    is_dynamic: bool,
}

#[derive(Serialize)]
struct RouteReport {
    route: String,
    file: String,
    fetches: Vec<FetchOccurrence>,
}

#[derive(Serialize, Default)]
struct FinalReport {
    summary: Summary,
    routes: Vec<RouteReport>,
    duplicates: Vec<FetchOccurrence>,
    unsupported: Vec<FetchOccurrence>,
}

#[derive(Serialize, Default)]
struct Summary {
    total_routes: usize,
    total_fetches: usize,
}

struct Cache {
    files: HashMap<PathBuf, Vec<FetchOccurrence>>,
}

struct FetchVisitor<'a> {
    source: &'a str,
    file: String,
    fetches: Vec<FetchOccurrence>,
}

impl<'a> Visit<'a> for FetchVisitor<'a> {
    fn visit_call_expression(&mut self, expr: &CallExpression<'a>) {
        if let Expression::Identifier(ident) = &expr.callee {
            if ident.name == "fetch" {
                let mut url = "unknown".to_string();
                let mut method = "GET".to_string();
                let mut is_dynamic = false;

                if let Some(arg) = expr.arguments.first() {
                    let result = extract_url_from_argument(arg, self.source);
                    url = result.0;
                    is_dynamic = result.1;
                }

                if let Some(Argument::ObjectExpression(obj)) = expr.arguments.get(1) {
                    for prop in &obj.properties {
                        if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) = prop {
                            if let Some(name) = p.key.static_name() {
                                if name == "method" {
                                    if let Expression::StringLiteral(s) = &p.value {
                                        method = s.value.to_string();
                                    }
                                }
                            }
                        }
                    }
                }

                self.fetches.push(FetchOccurrence {
                    url,
                    method,
                    file: self.file.clone(),
                    is_dynamic,
                });
            }
        }
        walk::walk_call_expression(self, expr);
    }
}

fn extract_url_from_argument(arg: &Argument, source: &str) -> (String, bool) {
    match arg {
        Argument::StringLiteral(s) => (s.value.to_string(), false),
        Argument::TemplateLiteral(t) => {
            let is_dynamic = !t.expressions.is_empty();
            (ast::template_literal_text(t, source), is_dynamic)
        }
        _ => ("dynamic".to_string(), true),
    }
}

fn main() -> Result<()> {
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
    let routes = routes::collect_routes(&frontend_root, &stems)?;

    let mut cache = Cache {
        files: HashMap::new(),
    };
    let mut reports = Vec::new();
    let mut global_fetches = Vec::new();

    for route in routes {
        // Filter by targets if provided
        if !cli.targets.is_empty() {
            let matched = cli.targets.iter().any(|t| {
                route.pattern == *t
                    || route.file.to_string_lossy().contains(t)
                    || route.pattern.starts_with(t)
            });
            if !matched {
                continue;
            }
        }

        let mut fetches = Vec::new();
        let mut visited = HashSet::new();

        // Analyze the page/route file itself
        analyze_file(&route.file, &root, &mut visited, &mut fetches, &mut cache)?;

        // Traverse up and find parent layouts/loadings if it's a page (UI)
        if route.file.file_stem().and_then(|s| s.to_str()) == Some("page") {
            let mut current = route.file.parent();
            while let Some(parent) = current {
                if !parent.starts_with(&frontend_root) {
                    break;
                }

                for stem in ["layout", "loading", "error", "not-found"] {
                    for ext in ["tsx", "ts", "jsx", "js"] {
                        let layout_file = parent.join(format!("{stem}.{ext}"));
                        if layout_file.exists() {
                            analyze_file(
                                &layout_file,
                                &root,
                                &mut visited,
                                &mut fetches,
                                &mut cache,
                            )?;
                        }
                    }
                }
                current = parent.parent();
            }
        }

        fetches.sort();
        fetches.dedup();
        global_fetches.extend(fetches.clone());

        reports.push(RouteReport {
            route: route.pattern,
            file: relative_string(&root, &route.file),
            fetches,
        });
    }

    let mut final_report = FinalReport {
        summary: Summary {
            total_routes: reports.len(),
            total_fetches: global_fetches.len(),
        },
        routes: reports,
        ..Default::default()
    };

    // Calculate duplicates and unsupported
    let mut counts = HashMap::new();
    for f in &global_fetches {
        let key = (f.method.clone(), f.url.clone());
        *counts.entry(key).or_insert(0) += 1;
        if f.is_dynamic {
            final_report.unsupported.push(f.clone());
        }
    }
    final_report.unsupported.sort();
    final_report.unsupported.dedup();

    for ((method, url), count) in counts {
        if count > 1 {
            // Find one example occurrence
            if let Some(f) = global_fetches
                .iter()
                .find(|f| f.method == method && f.url == url)
            {
                final_report.duplicates.push(f.clone());
            }
        }
    }
    final_report.duplicates.sort();

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&final_report)?);
    } else {
        print_markdown_report(&final_report);
    }

    Ok(())
}

fn analyze_file(
    path: &Path,
    root: &Path,
    visited: &mut HashSet<PathBuf>,
    fetches: &mut Vec<FetchOccurrence>,
    cache: &mut Cache,
) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let abs_path = path.canonicalize()?;
    if visited.contains(&abs_path) {
        return Ok(());
    }
    visited.insert(abs_path.clone());

    if let Some(cached_fetches) = cache.files.get(&abs_path) {
        fetches.extend(cached_fetches.clone());
        return Ok(());
    }

    let source = std::fs::read_to_string(&abs_path)?;
    let rel_file = relative_string(root, &abs_path);

    let mut file_fetches = Vec::new();
    let _ = ast::with_program(path, &source, |program, source| -> Result<()> {
        let mut visitor = FetchVisitor {
            source,
            file: rel_file,
            fetches: Vec::new(),
        };
        visitor.visit_program(program);
        file_fetches.extend(visitor.fetches);

        // Find imports and recurse
        for stmt in &program.body {
            if let oxc_ast::ast::Statement::ImportDeclaration(import) = stmt {
                let specifier = import.source.value.as_str();
                if let Some(resolved) = resolve_import(path, specifier) {
                    analyze_file(&resolved, root, visited, &mut file_fetches, cache)?;
                }
            }
        }
        Ok(())
    })??;

    cache.files.insert(abs_path, file_fetches.clone());
    fetches.extend(file_fetches);

    Ok(())
}

fn resolve_import(current_file: &Path, specifier: &str) -> Option<PathBuf> {
    if specifier.starts_with('.') {
        let parent = current_file.parent()?;
        let joined = parent.join(specifier);
        if joined.exists() && joined.is_file() {
            return Some(joined);
        }
        for ext in ["tsx", "ts", "jsx", "js"] {
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
    println!("- Total Unique Fetches: {}", report.summary.total_fetches);
    println!();

    println!("## Routes");
    for route in &report.routes {
        println!("### {} ({})", route.route, route.file);
        if route.fetches.is_empty() {
            println!("(no fetches found)");
        } else {
            println!("| Method | URL | File | Dynamic |");
            println!("| --- | --- | --- | --- |");
            for fetch in &route.fetches {
                println!(
                    "| {} | `{}` | {} | {} |",
                    fetch.method,
                    fetch.url,
                    fetch.file,
                    if fetch.is_dynamic { "✅" } else { "❌" }
                );
            }
        }
        println!();
    }

    if !report.duplicates.is_empty() {
        println!("## Duplicates");
        println!("| Method | URL | Example File |");
        println!("| --- | --- | --- |");
        for fetch in &report.duplicates {
            println!("| {} | `{}` | {} |", fetch.method, fetch.url, fetch.file);
        }
        println!();
    }

    if !report.unsupported.is_empty() {
        println!("## Unsupported (Dynamic)");
        println!("| Method | URL | File |");
        println!("| --- | --- | --- |");
        for fetch in &report.unsupported {
            println!("| {} | `{}` | {} |", fetch.method, fetch.url, fetch.file);
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_extract_string_literal_from_argument_none() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "fetch(true)";
        let source_type = oxc_span::SourceType::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
        let stmt = &parsed.program.body[0];
        if let oxc_ast::ast::Statement::ExpressionStatement(expr_stmt) = stmt {
            if let Expression::CallExpression(call) = &expr_stmt.expression {
                let arg = &call.arguments[0];
                assert_eq!(
                    extract_url_from_argument(arg, source),
                    ("dynamic".to_string(), true)
                );
            }
        }
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
        };
        visitor.visit_program(&parsed.program);
        assert_eq!(visitor.fetches.len(), 5);
        for fetch in &visitor.fetches {
            assert_eq!(fetch.method, "GET");
            assert!(fetch.is_dynamic);
        }
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
        };
        visitor.visit_program(&parsed.program);
        assert_eq!(visitor.fetches.len(), 1);
        assert_eq!(visitor.fetches[0].url, "unknown");
        assert!(!visitor.fetches[0].is_dynamic);
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
        };
        visitor.visit_program(&parsed.program);
        assert_eq!(visitor.fetches.len(), 3);
        assert_eq!(visitor.fetches[0].url, "dynamic");
        assert!(visitor.fetches[0].is_dynamic);
        assert_eq!(visitor.fetches[1].url, "/api/${id}");
        assert!(visitor.fetches[1].is_dynamic);
        assert_eq!(visitor.fetches[1].method, "PATCH");
        assert_eq!(visitor.fetches[2].url, "/api/get");
        assert!(!visitor.fetches[2].is_dynamic);
        assert_eq!(visitor.fetches[2].method, "GET");
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
    fn test_resolve_import_root() {
        assert_eq!(resolve_import(Path::new("page.ts"), "./lib"), None);
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
        };
        let mut visited = HashSet::new();
        let mut fetches = Vec::new();
        analyze_file(&file, dir.path(), &mut visited, &mut fetches, &mut cache).unwrap();
        assert_eq!(fetches.len(), 1);

        let mut visited2 = HashSet::new();
        let mut fetches2 = Vec::new();
        analyze_file(&file, dir.path(), &mut visited2, &mut fetches2, &mut cache).unwrap();
        assert_eq!(fetches2.len(), 1);
        assert_eq!(fetches2[0].url, "/api/cache");
    }

    #[test]
    fn test_analyze_file_not_exists() {
        let mut visited = HashSet::new();
        let mut fetches = Vec::new();
        let mut cache = Cache {
            files: HashMap::new(),
        };
        analyze_file(
            Path::new("missing.ts"),
            Path::new("."),
            &mut visited,
            &mut fetches,
            &mut cache,
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
        visited.insert(file.canonicalize().unwrap());
        let mut fetches = Vec::new();
        let mut cache = Cache {
            files: HashMap::new(),
        };
        analyze_file(&file, dir.path(), &mut visited, &mut fetches, &mut cache).unwrap();
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
        };
        let err = analyze_file(&path, dir.path(), &mut visited, &mut fetches, &mut cache)
            .err()
            .unwrap();
        assert!(
            err.to_string().contains("failed to read")
                || err.to_string().contains("Is a directory")
        );
    }
}
