use super::*;
use no_mistakes_core::config::v2::schema::{Project, ProjectType, RuleDef, RuleScope};

fn fixture(path: &str) -> PathBuf {
    no_mistakes_core::codebase::ts_resolver::normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(path),
    )
}

fn unique_exports_rule(projects: Vec<&str>) -> RuleDef {
    RuleDef {
        rule: no_mistakes_core::codebase::unique_exports::RULE_ID.to_string(),
        projects: projects.into_iter().map(str::to_string).collect(),
        ..Default::default()
    }
}

#[test]
fn unique_exports_project_roots_cover_target_variants() {
    let root = fixture("config-v2/nextjs-inferred-root");
    let mut config = NoMistakesConfig::default();
    config.projects.insert(
        "web".to_string(),
        Project {
            type_: Some(ProjectType::Nextjs),
            ..Default::default()
        },
    );
    config.projects.insert(
        "backend".to_string(),
        Project {
            root: Some("backend".to_string()),
            ..Default::default()
        },
    );
    config
        .projects
        .insert("repo".to_string(), Project::default());
    config.rules.push(unique_exports_rule(vec![
        "missing", "web", "backend", "repo",
    ]));
    config.rules.push(RuleDef {
        rule: no_mistakes_core::codebase::unique_exports::RULE_ID.to_string(),
        scope: Some(RuleScope::Repository),
        ..Default::default()
    });

    let roots = unique_exports_project_roots(&root, &config);

    assert_eq!(
        roots,
        vec![root.clone(), root.join("backend"), root.join("web")]
    );
}

#[test]
fn discover_check_files_includes_inferred_nextjs_project_files() {
    let root = fixture("config-v2/nextjs-inferred-root");
    let config: NoMistakesConfig =
        serde_yaml::from_str(&std::fs::read_to_string(root.join(".no-mistakes.yml")).unwrap())
            .unwrap();

    let files = discover_check_files(&root, &config, &[], true);

    assert!(files.iter().any(|path| path.ends_with("web/app/page.tsx")));
}

#[test]
fn nextjs_project_without_single_config_root_is_ignored() {
    let root = fixture("config-v2/empty");
    let mut config = NoMistakesConfig::default();
    config.projects.insert(
        "web".to_string(),
        Project {
            type_: Some(ProjectType::Nextjs),
            ..Default::default()
        },
    );
    config.rules.push(unique_exports_rule(vec!["web"]));

    let roots = unique_exports_project_roots(&root, &config);

    assert!(roots.is_empty());
}
