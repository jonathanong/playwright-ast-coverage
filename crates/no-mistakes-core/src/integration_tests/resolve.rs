use super::types::{
    CallTarget, FileAnalysis, FunctionInfo, FunctionKey, ImportBinding, ImportedName,
};
use crate::codebase::ts_resolver::{resolve_import, TsConfig};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

pub(super) struct ImportResolution<'a> {
    pub analyses: &'a BTreeMap<PathBuf, FileAnalysis>,
    pub export_index: &'a HashMap<(PathBuf, String), FunctionKey>,
    pub tsconfig: &'a TsConfig,
}

pub(super) fn build_function_index(
    analyses: &BTreeMap<PathBuf, FileAnalysis>,
) -> HashMap<FunctionKey, FunctionInfo> {
    let mut index = HashMap::new();
    for (file, analysis) in analyses {
        for (name, info) in &analysis.functions {
            index.insert(
                FunctionKey {
                    file: file.clone(),
                    name: name.clone(),
                },
                info.clone(),
            );
        }
    }
    index
}

pub(super) fn build_export_index(
    analyses: &BTreeMap<PathBuf, FileAnalysis>,
) -> HashMap<(PathBuf, String), FunctionKey> {
    let mut index = HashMap::new();
    for (file, analysis) in analyses {
        for (exported, local) in &analysis.exports {
            index.insert(
                (file.clone(), exported.clone()),
                FunctionKey {
                    file: file.clone(),
                    name: local.clone(),
                },
            );
        }
    }
    index
}

pub(super) fn resolved_integrations(
    root: &FunctionKey,
    function_index: &HashMap<FunctionKey, FunctionInfo>,
    resolver: &ImportResolution<'_>,
) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut integrations = Vec::new();
    resolved_integrations_inner(root, function_index, resolver, &mut seen, &mut integrations);
    integrations.sort();
    integrations.dedup();
    integrations
}

fn resolved_integrations_inner(
    key: &FunctionKey,
    function_index: &HashMap<FunctionKey, FunctionInfo>,
    resolver: &ImportResolution<'_>,
    seen: &mut HashSet<FunctionKey>,
    integrations: &mut Vec<String>,
) {
    if !seen.insert(key.clone()) {
        return;
    }
    let Some(info) = function_index.get(key) else {
        return;
    };
    if let Some(integration) = &info.integration {
        integrations.push(integration.clone());
    }
    for call in &info.calls {
        let Some(next) = resolve_call(key, call, resolver) else {
            continue;
        };
        resolved_integrations_inner(&next, function_index, resolver, seen, integrations);
    }
}

fn resolve_call(
    caller: &FunctionKey,
    call: &CallTarget,
    resolver: &ImportResolution<'_>,
) -> Option<FunctionKey> {
    match call {
        CallTarget::Local(name) => {
            let analysis = resolver.analyses.get(&caller.file)?;
            if let Some(binding) = analysis.imports.get(name) {
                return resolve_import_binding(&caller.file, binding, resolver);
            }
            Some(FunctionKey {
                file: caller.file.clone(),
                name: name.clone(),
            })
        }
        CallTarget::Imported { local } => {
            let analysis = resolver.analyses.get(&caller.file)?;
            let binding = analysis.imports.get(local)?;
            resolve_import_binding(&caller.file, binding, resolver)
        }
        CallTarget::Namespace { namespace, member } => {
            let analysis = resolver.analyses.get(&caller.file)?;
            let binding = analysis.imports.get(namespace)?;
            let ImportedName::Namespace = binding.imported else {
                return None;
            };
            let resolved_file = resolve_import(&binding.source, &caller.file, resolver.tsconfig)?;
            resolver
                .export_index
                .get(&(resolved_file, member.clone()))
                .cloned()
        }
    }
}

fn resolve_import_binding(
    caller_file: &Path,
    binding: &ImportBinding,
    resolver: &ImportResolution<'_>,
) -> Option<FunctionKey> {
    let resolved_file = resolve_import(&binding.source, caller_file, resolver.tsconfig)?;
    let imported_name = match &binding.imported {
        ImportedName::Named(name) => name.clone(),
        ImportedName::Default => "default".to_string(),
        ImportedName::Namespace => return None,
    };
    resolver
        .export_index
        .get(&(resolved_file, imported_name))
        .cloned()
}
