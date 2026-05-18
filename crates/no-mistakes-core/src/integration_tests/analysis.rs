use super::calls;
use super::types::{FileAnalysis, FunctionInfo, FunctionKey, TestCase};
use crate::ast;
use analysis_helpers::FunctionBodySpan;
use anyhow::{Context, Result};
use oxc_ast::ast::{BindingPattern, ModuleExportName, Program, Statement};
use oxc_ast_visit::{walk, Visit};
use oxc_span::{GetSpan, Span};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod analysis_helpers;

pub(super) fn analyze_files(files: &[PathBuf]) -> Result<BTreeMap<PathBuf, FileAnalysis>> {
    let mut analyses = BTreeMap::new();
    for file in files {
        let source = match std::fs::read_to_string(file) {
            Ok(source) => source,
            Err(_) => continue,
        };
        let analysis = ast::with_program(file, &source, |program, source| {
            analyze_program(file, program, source)
        })
        .with_context(|| format!("analyzing integration annotations in {}", file.display()))?;
        analyses.insert(file.clone(), analysis);
    }
    Ok(analyses)
}

pub(crate) fn analyze_program(path: &Path, program: &Program<'_>, source: &str) -> FileAnalysis {
    let mut collector = AnalysisCollector {
        path,
        source,
        result: FileAnalysis::default(),
        describe_stack: Vec::new(),
        statement_depth: 0,
        test_counter: 0,
    };
    collector.visit_program(program);
    collector.result
}

struct AnalysisCollector<'a, 'p> {
    path: &'p Path,
    source: &'a str,
    result: FileAnalysis,
    describe_stack: Vec<String>,
    statement_depth: usize,
    test_counter: usize,
}

impl<'a> Visit<'a> for AnalysisCollector<'a, '_> {
    fn visit_import_declaration(&mut self, import: &oxc_ast::ast::ImportDeclaration<'a>) {
        let source = import.source.value.to_string();
        if let Some(specifiers) = import.specifiers.as_ref() {
            for specifier in specifiers {
                self.collect_import(specifier, &source);
            }
        }
    }

    fn visit_statement(&mut self, statement: &Statement<'a>) {
        if self.statement_depth == 0 {
            match statement {
                Statement::FunctionDeclaration(function) => {
                    if let Some(id) = &function.id {
                        self.collect_function(
                            id.name.as_str(),
                            function.span(),
                            function.body_span(),
                            false,
                        );
                    }
                }
                Statement::VariableDeclaration(declaration) => {
                    for declarator in &declaration.declarations {
                        let Some(name) = binding_name(&declarator.id) else {
                            continue;
                        };
                        self.collect_init_function(name, declarator.init.as_ref());
                    }
                }
                Statement::ExportNamedDeclaration(export) => {
                    if let Some(declaration) = &export.declaration {
                        self.collect_export_declaration(declaration);
                    }
                    for specifier in &export.specifiers {
                        self.result.exports.insert(
                            module_export_name(&specifier.exported),
                            module_export_name(&specifier.local),
                        );
                    }
                }
                Statement::ExportDefaultDeclaration(export) => self.collect_default_export(export),
                _ => {}
            }
        }
        self.statement_depth += 1;
        walk::walk_statement(self, statement);
        self.statement_depth -= 1;
    }

    fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
        if calls::is_skipped_describe(call) {
            return;
        }
        if let Some(name) = calls::describe_name(call) {
            self.describe_stack.push(name);
            walk::walk_call_expression(self, call);
            self.describe_stack.pop();
            return;
        }
        if let Some(test_name) = calls::test_name(call) {
            if let Some((argument, span)) = calls::callback_argument(call) {
                self.collect_test(call.span.start, test_name, argument, span);
                return;
            }
        }
        walk::walk_call_expression(self, call);
    }
}

impl AnalysisCollector<'_, '_> {
    fn collect_test(
        &mut self,
        call_start: u32,
        test_name: String,
        argument: &oxc_ast::ast::Argument<'_>,
        span: Span,
    ) {
        self.test_counter += 1;
        let function_name = format!("__test_{}", self.test_counter);
        self.result.functions.insert(
            function_name.clone(),
            FunctionInfo {
                integration: calls::integration_annotation_before(self.source, span),
                calls: calls::collect_calls(argument),
            },
        );
        self.result.tests.push(TestCase {
            name: Some(test_name),
            describe_path: self.describe_stack.clone(),
            function_key: FunctionKey {
                file: self.path.to_path_buf(),
                name: function_name,
            },
            line: crate::codebase::ts_source::byte_offset_to_line(self.source, call_start as usize),
        });
    }
}

fn binding_name<'a>(pattern: &'a BindingPattern<'a>) -> Option<&'a str> {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

fn module_export_name(name: &ModuleExportName<'_>) -> String {
    match name {
        ModuleExportName::IdentifierName(id) => id.name.to_string(),
        ModuleExportName::IdentifierReference(id) => id.name.to_string(),
        ModuleExportName::StringLiteral(value) => value.value.to_string(),
    }
}
