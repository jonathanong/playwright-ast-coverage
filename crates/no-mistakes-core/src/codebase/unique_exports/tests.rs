use super::*;
use crate::codebase::ts_resolver::normalize_path;
use crate::codebase::workspaces::WorkspaceMap;

fn fixture(name: &str) -> PathBuf {
    normalize_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codebase-analysis")
            .join(name),
    )
}

fn findings(name: &str) -> Vec<UniqueExportFinding> {
    analyze_project(&fixture(name), None, None).unwrap()
}

fn finding_names(findings: &[UniqueExportFinding]) -> Vec<(String, String)> {
    findings
        .iter()
        .map(|finding| (finding.export_name.clone(), finding.export_kind.clone()))
        .collect()
}

#[test]
fn reports_duplicate_value_and_type_exports_separately() {
    let findings = findings("unique-exports-basic");
    assert_eq!(findings.len(), 2);
    assert!(findings
        .iter()
        .any(|f| f.export_name == "shared" && f.export_kind == "value"));
    assert!(findings
        .iter()
        .any(|f| f.export_name == "SharedType" && f.export_kind == "type"));
    assert!(!findings.iter().any(|f| f.export_name == "default"));
}

#[test]
fn strict_mode_reports_cross_type_duplicates() {
    let findings = findings("unique-exports-strict");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].export_name, "Shared");
    assert_eq!(findings[0].export_kind, "export");
    assert!(findings[0].message.starts_with("export `Shared`"));
    assert!(!findings[0].message.contains("export export"));
}

#[test]
fn follows_explicit_and_star_reexports() {
    let findings = findings("unique-exports-reexports");
    let alpha = findings
        .iter()
        .filter(|f| f.export_name == "Alpha" && f.export_kind == "value")
        .count();
    let beta = findings
        .iter()
        .filter(|f| f.export_name == "Beta" && f.export_kind == "type")
        .count();
    assert_eq!(alpha, 2);
    assert_eq!(beta, 2);
}

#[test]
fn honors_rule_disable_comments() {
    let findings = findings("unique-exports-disabled");
    assert!(findings.is_empty());
}

#[test]
fn exempts_known_nextjs_framework_exports_only_in_convention_files() {
    let findings = findings("unique-exports-nextjs");
    let metadata_count = findings
        .iter()
        .filter(|finding| finding.export_name == "metadata")
        .count();
    assert_eq!(metadata_count, 1);
    assert!(findings
        .iter()
        .any(|finding| finding.export_name == "metadata"
            && finding.file.starts_with("web/components/")));
    assert!(
        findings
            .iter()
            .any(|finding| finding.export_name == "runtime"
                && finding.file.ends_with("page.test.tsx"))
    );
}

#[test]
fn checks_framework_named_exports_outside_nextjs_projects() {
    let findings = findings("unique-exports-not-next-app");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].export_name, "metadata");
    assert!(nextjs::is_framework_export(
        "web/app/page",
        "metadata",
        true
    ));

    let next_root = fixture("unique-exports-nextjs");
    assert!(scan::package_json_has_next_dependency(
        &next_root.join("package.json")
    ));
    assert!(scan::file_is_in_nextjs_project(
        &next_root,
        &next_root.join("web/app/users/page.tsx")
    ));

    let not_next_root = fixture("unique-exports-not-next-app");
    assert!(!scan::file_is_in_nextjs_project(
        &not_next_root,
        Path::new("")
    ));
    assert!(!scan::package_json_has_next_dependency(
        &fixture("unique-exports-not-next-deps").join("package.json")
    ));
}

#[test]
fn checks_across_workspace_packages() {
    let findings = findings("unique-exports-workspace");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].export_name, "WorkspaceDuplicate");
}

#[test]
fn disabled_config_skips_rule() {
    assert!(findings("unique-exports-config-disabled").is_empty());
}

#[test]
fn explicit_tsconfig_resolves_path_aliases() {
    let root = fixture("unique-exports-tsconfig-paths");
    let findings = analyze_project(&root, None, Some(&root.join("tsconfig.json"))).unwrap();
    assert_eq!(findings.len(), 2);
    assert!(findings
        .iter()
        .all(|finding| finding.export_name == "ViaConfig"));
}

#[test]
fn relative_explicit_tsconfig_resolves_from_project_root() {
    let root = fixture("unique-exports-tsconfig-paths");
    let findings = analyze_project(&root, None, Some(Path::new("tsconfig.json"))).unwrap();

    assert_eq!(findings.len(), 2);
    assert!(findings
        .iter()
        .all(|finding| finding.export_name == "ViaConfig"));
}

#[test]
fn nearest_tsconfig_is_discovered_and_explicit_errors_are_reported() {
    let root = fixture("unique-exports-tsconfig-paths");
    let findings = analyze_project(&root, None, None).unwrap();
    assert!(findings
        .iter()
        .any(|finding| finding.export_name == "ViaConfig"));
    assert!(analyze_project(&root, None, Some(&root.join("missing-tsconfig.json"))).is_err());
}

#[test]
fn covers_reexport_resolution_edge_cases() {
    let findings = findings("unique-exports-edge-cases");
    let names = finding_names(&findings);
    assert!(names.contains(&("Direct".to_string(), "value".to_string())));
    assert!(names.contains(&("DirectType".to_string(), "type".to_string())));
    assert!(names.contains(&("DefaultAlias".to_string(), "value".to_string())));
    assert!(names.contains(&("DefaultShapeAlias".to_string(), "type".to_string())));
    assert!(names.contains(&("ChainAlias".to_string(), "type".to_string())));
    assert!(names.contains(&("StarResolved".to_string(), "value".to_string())));
    assert!(names.contains(&("TypeStarOnly".to_string(), "type".to_string())));
    assert!(!names.contains(&("TypeStarValue".to_string(), "value".to_string())));
    assert!(names.contains(&("Namespace".to_string(), "value".to_string())));
    assert!(!names.contains(&("NamespacedOnly".to_string(), "value".to_string())));
    assert!(!names.contains(&("default".to_string(), "value".to_string())));
    assert!(names.contains(&("Hidden".to_string(), "value".to_string())));
    assert!(names.contains(&("Skipped".to_string(), "value".to_string())));
    assert!(names.contains(&("SameLine".to_string(), "value".to_string())));
}

#[test]
fn scan_helpers_cover_filter_and_parse_edges() {
    let root = fixture("unique-exports-edge-cases");
    let files = vec![
        root.join("src/direct.ts"),
        root.join("src/invalid.ts"),
        root.join("src/not-present.ts"),
        root.join("package.json"),
    ];
    let filtered = scan::filter_source_files(&root, files.clone(), &["[".to_string()]);
    assert_eq!(filtered.len(), 3);

    let filtered = scan::filter_source_files(&root, files, &["invalid\\.ts$".to_string()]);
    assert_eq!(filtered.len(), 2);
    assert!(!filtered.iter().any(|path| path.ends_with("src/invalid.ts")));

    let sources = scan::collect_source_files(&root, &filtered);
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].rel, "src/direct.ts");

    let lookup = scan::NextJsProjectLookup::new(&fixture("unique-exports-nextjs"), &[]);
    assert!(!lookup.contains_file(&root.join("src/direct.ts")));
}

#[test]
fn defensive_helpers_ignore_missing_targets_and_non_matching_default_exports() {
    let root = fixture("unique-exports-edge-cases");
    let all_files = discover_files(&root, &[]);
    let source_files = scan::collect_source_files(&root, &all_files);
    let files: HashMap<PathBuf, SourceFile> = source_files
        .into_iter()
        .map(|file| (file.path.clone(), file))
        .collect();
    let tsconfig = crate::codebase::ts_resolver::TsConfig {
        dir: root.clone(),
        paths: Vec::new(),
        paths_dir: root.clone(),
        base_url: None,
    };
    let resolver = ImportResolver::new(&tsconfig);
    let workspace = WorkspaceMap::default();

    let mut visiting = HashSet::new();
    assert!(collector::collect_file_exports(
        &root.join("src/not-present.ts"),
        &files,
        &resolver,
        &workspace,
        &mut visiting,
    )
    .is_empty());

    let mut visiting = HashSet::new();
    assert_eq!(
        collector::find_target_export_bucket(
            &root.join("src/not-present.ts"),
            "Missing",
            &files,
            &resolver,
            &workspace,
            &mut visiting,
        ),
        None
    );

    let mut visiting = HashSet::new();
    assert_eq!(
        collector::find_target_export_bucket(
            &root.join("src/default-source.ts"),
            "NotDefault",
            &files,
            &resolver,
            &workspace,
            &mut visiting,
        ),
        None
    );
}
