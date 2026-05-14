use anyhow::Result;
use clap::Parser;
use no_mistakes_core::ast;
use no_mistakes_core::config;
use no_mistakes_core::routes;
use oxc_ast::ast::{Argument, CallExpression, Expression, Statement};
use oxc_ast_visit::{walk, Visit};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
struct FetchOccurrence {
    url: String,
    method: String,
    file: String,
}

#[derive(Serialize)]
struct RouteReport {
    route: String,
    file: String,
    fetches: Vec<FetchOccurrence>,
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

                if let Some(arg) = expr.arguments.first() {
                    url = extract_string_literal_from_argument(arg, self.source)
                        .unwrap_or_else(|| "dynamic".to_string());
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
                });
            }
        } walk::walk_call_expression(self, expr);
    }
}

fn extract_string_literal_from_argument(arg: &Argument, source: &str) -> Option<String> {
    match arg {
        Argument::StringLiteral(s) => Some(s.value.to_string()),
        Argument::TemplateLiteral(t) => Some(ast::template_literal_text(t, source)),
        _ => None,
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
    let stems = ["page", "route", "layout"];
    let routes = routes::collect_routes(&frontend_root, &stems)?;

    let mut reports = Vec::new();
    for route in routes {
        let mut fetches = Vec::new();
        let mut visited = HashSet::new();
        analyze_file(&route.file, &root, &mut visited, &mut fetches)?;

        fetches.sort();
        fetches.dedup();

        reports.push(RouteReport {
            route: route.pattern,
            file: relative_string(&root, &route.file),
            fetches,
        });
    }

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&reports)?);
    } else {
        print_text_report(&reports);
    }

    Ok(())
}

fn analyze_file(
    path: &Path,
    root: &Path,
    visited: &mut HashSet<PathBuf>,
    fetches: &mut Vec<FetchOccurrence>,
) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let abs_path = path.canonicalize()?;
    if visited.contains(&abs_path) {
        return Ok(());
    }
    visited.insert(abs_path.clone());

    let source = std::fs::read_to_string(&abs_path)?;
    let rel_file = relative_string(root, &abs_path);

    ast::with_program(path, &source, |program, source| {
        let mut visitor = FetchVisitor {
            source,
            file: rel_file,
            fetches: Vec::new(),
        };
        visitor.visit_program(program);
        fetches.extend(visitor.fetches);

        // Find imports and recurse
        for stmt in &program.body {
            if let Statement::ImportDeclaration(import) = stmt {
                let specifier = import.source.value.as_str();
                if let Some(resolved) = resolve_import(path, specifier) {
                    let _ = analyze_file(&resolved, root, visited, fetches);
                }
            }
        }
    })?;

    Ok(())
}

fn resolve_import(current_file: &Path, specifier: &str) -> Option<PathBuf> {
    if specifier.starts_with('.') {
        let parent = current_file.parent()?;
        let joined = parent.join(specifier);
        let extensions = ["tsx", "ts", "jsx", "js"];
        for ext in extensions {
            let path = joined.with_extension(ext);
            if path.exists() {
                return Some(path);
            }
            let index = joined.join(format!("index.{ext}"));
            if index.exists() {
                return Some(index);
            }
        }
    } None
}

fn relative_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn print_text_report(reports: &[RouteReport]) {
    for report in reports {
        println!("Route: {} ({})", report.route, report.file);
        if report.fetches.is_empty() {
            println!("  (no fetches found)");
        } else {
            for fetch in &report.fetches {
                println!("  {} {}", fetch.method, fetch.url);
            }
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
        let Statement::ExpressionStatement(expr_stmt) = stmt else { unreachable!() };
        let Expression::CallExpression(call) = &expr_stmt.expression else { unreachable!() };
        let arg = &call.arguments[0];
        assert_eq!(extract_string_literal_from_argument(arg, source), None);
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
        assert_eq!(visitor.fetches[1].url, "/api/${id}");
        assert_eq!(visitor.fetches[1].method, "PATCH");
        assert_eq!(visitor.fetches[2].url, "/api/get");
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
    fn test_resolve_import_none() {
        let dir = tempdir().unwrap();
        let current = dir.path().join("main.ts");
        assert_eq!(resolve_import(&current, "./missing"), None);
    }

    #[test]
    fn test_analyze_file_not_exists() {
        let mut visited = HashSet::new();
        let mut fetches = Vec::new();
        analyze_file(
            Path::new("missing.ts"),
            Path::new("."),
            &mut visited,
            &mut fetches,
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
        analyze_file(&file, dir.path(), &mut visited, &mut fetches).unwrap();
        assert!(fetches.is_empty());
    }

    #[test]
    fn test_analyze_file_read_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("dir.ts");
        fs::create_dir(&path).unwrap();
        let mut visited = HashSet::new();
        let mut fetches = Vec::new();
        let err = analyze_file(&path, dir.path(), &mut visited, &mut fetches)
            .err()
            .unwrap();
        assert!(
            err.to_string().contains("failed to read")
                || err.to_string().contains("Is a directory")
        );
    }
}
