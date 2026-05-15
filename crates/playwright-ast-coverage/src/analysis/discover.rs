use crate::analysis::context::{DiscoveredTestFile, TestProjectContext, TestProjectDiscovery};
use crate::config::Settings;
use crate::fsutil::{build_globset, relative_string, slash_path, walk_files};
use crate::playwright_config;
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

pub(crate) fn discover_test_files(
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

pub(crate) fn build_project_discovery(
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

pub(crate) fn test_project_contexts(projects: &[TestProjectDiscovery]) -> Vec<TestProjectContext> {
    let mut contexts: Vec<TestProjectContext> = projects
        .iter()
        .map(|project| project.context.clone())
        .collect();
    contexts.sort();
    contexts.dedup();
    contexts
}

pub(crate) fn matching_project_contexts(
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
