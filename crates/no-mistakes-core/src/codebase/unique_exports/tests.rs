use super::*;
use crate::codebase::ts_resolver::normalize_path;
use crate::codebase::workspaces::WorkspaceMap;

#[path = "tests/origin.rs"]
mod origin;
#[path = "tests/shared_facts_disable.rs"]
mod shared_facts_disable;

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
fn analyzes_project_from_shared_facts() {
    let root = fixture("unique-exports-basic");
    let files = crate::codebase::ts_source::discover_files(&root, &[]);
    let facts = crate::codebase::check_facts::collect_check_facts(
        &root,
        files,
        crate::codebase::check_facts::CheckFactPlan {
            symbols: true,
            source: true,
            ..Default::default()
        },
    );

    let tsconfig = Path::new("tsconfig.json");
    let findings = analyze_project_with_facts(&root, None, Some(tsconfig), &facts).unwrap();

    assert_eq!(findings.len(), 2);
}

#[test]
fn analyzes_nextjs_project_from_shared_facts() {
    let root = fixture("unique-exports-nextjs");
    let files = crate::codebase::ts_source::discover_files(&root, &[]);
    let facts = crate::codebase::check_facts::collect_check_facts(
        &root,
        files,
        crate::codebase::check_facts::CheckFactPlan {
            symbols: true,
            source: true,
            ..Default::default()
        },
    );

    let findings = analyze_project_with_facts(&root, None, None, &facts).unwrap();

    assert!(findings
        .iter()
        .any(|finding| finding.export_name == "metadata"));
}

#[test]
fn analyze_project_with_facts_returns_empty_without_enabled_projects() {
    let root = fixture("unique-exports-config-disabled");
    let facts = crate::codebase::check_facts::CheckFactMap::default();

    let findings = analyze_project_with_facts(&root, None, None, &facts).unwrap();

    assert!(findings.is_empty());
}

#[test]
fn analyze_project_with_facts_honors_disable_comments() {
    let root = fixture("unique-exports-disabled");
    let files = crate::codebase::ts_source::discover_files(&root, &[]);
    let facts = crate::codebase::check_facts::collect_check_facts(
        &root,
        files,
        crate::codebase::check_facts::CheckFactPlan {
            symbols: true,
            source: true,
            ..Default::default()
        },
    );

    let findings = analyze_project_with_facts(&root, None, None, &facts).unwrap();

    assert!(findings.is_empty());
}

#[test]
fn collect_source_files_from_facts_reports_missing_fact_shapes() {
    let root = fixture("unique-exports-basic");
    let file = root.join("src/a.ts");
    let files = vec![file.clone()];
    let missing = crate::codebase::check_facts::CheckFactMap::default();

    assert!(
        scan::collect_source_files_from_facts(&root, &files, &missing)
            .unwrap_err()
            .to_string()
            .contains("missing shared facts")
    );

    let mut parse_error = crate::codebase::check_facts::CheckFactMap::default();
    parse_error.ts.insert(
        file.clone(),
        crate::codebase::check_facts::CheckFileFacts {
            source: Some("export const Broken =".to_string()),
            parse_error: Some("bad syntax".to_string()),
            ..Default::default()
        },
    );
    assert!(
        scan::collect_source_files_from_facts(&root, &files, &parse_error)
            .unwrap_err()
            .to_string()
            .contains("bad syntax")
    );

    let mut missing_source = crate::codebase::check_facts::CheckFactMap::default();
    missing_source.ts.insert(file.clone(), Default::default());
    assert!(
        scan::collect_source_files_from_facts(&root, &files, &missing_source)
            .unwrap_err()
            .to_string()
            .contains("missing source facts")
    );

    let mut missing_symbols = crate::codebase::check_facts::CheckFactMap::default();
    missing_symbols.ts.insert(
        file,
        crate::codebase::check_facts::CheckFileFacts {
            source: Some("export const value = 1;".to_string()),
            ..Default::default()
        },
    );
    assert!(
        scan::collect_source_files_from_facts(&root, &files, &missing_symbols)
            .unwrap_err()
            .to_string()
            .contains("missing symbol facts")
    );
}

#[test]
fn root_is_normalized_before_analysis() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codebase-analysis/unique-exports-basic/.");

    let findings = analyze_project(&root, None, None).unwrap();

    assert_eq!(findings.len(), 2);
    assert!(findings
        .iter()
        .all(|finding| !finding.file.starts_with('/')));
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
    assert!(findings.is_empty());
}

#[test]
fn collapses_source_declarations_and_same_origin_barrels() {
    let findings = findings("unique-exports-barrels-pass");
    assert!(findings.is_empty());
}

#[test]
fn reports_distinct_declarations_even_when_reexported_through_barrels() {
    let findings = findings("unique-exports-real-duplicates");
    let names = finding_names(&findings);
    assert_eq!(findings.len(), 2);
    assert!(names.contains(&("Collision".to_string(), "value".to_string())));
    assert!(names.contains(&("Shape".to_string(), "type".to_string())));
}

#[test]
fn checks_only_projects_that_enable_the_rule() {
    let findings = findings("unique-exports-project-scope");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].export_name, "ScopedDuplicate");
    assert!(!findings
        .iter()
        .any(|finding| finding.export_name == "IgnoredDuplicate"));
}

#[test]
fn top_level_disabled_rule_overrides_project_scopes() {
    assert!(findings("unique-exports-project-scope-disabled").is_empty());
}

#[test]
fn keeps_type_and_value_exports_separate_by_default() {
    assert!(findings("unique-exports-type-value-split").is_empty());
}

#[test]
fn strict_mode_still_reports_cross_type_duplicates_after_origin_deduping() {
    let findings = findings("unique-exports-type-value-strict");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].export_name, "Shared");
    assert_eq!(findings[0].export_kind, "export");
}

#[test]
fn collapses_workspace_barrels_to_their_source_export() {
    assert!(findings("unique-exports-workspace-barrels").is_empty());
}

#[test]
fn project_scoping_preserves_workspace_resolution_outside_enabled_roots() {
    assert!(findings("unique-exports-project-scope-workspace").is_empty());
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
    assert_eq!(metadata_count, 3);
    assert!(findings
        .iter()
        .any(|finding| finding.export_name == "metadata"
            && finding.file.starts_with("web/components/")));
    assert!(findings
        .iter()
        .any(|finding| finding.export_name == "metadata"
            && finding.file.starts_with("web/pages/app/")));
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
    assert!(!nextjs::is_framework_export(
        "web/pages/app/page.tsx",
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
fn exempts_nextjs_metadata_asset_convention_exports() {
    let findings = findings("unique-exports-nextjs-assets");
    assert!(findings.iter().any(|finding| finding.export_name == "alt"));
    assert!(findings.iter().any(|finding| finding.export_name == "size"));
    assert!(findings
        .iter()
        .any(|finding| finding.export_name == "contentType"));
    assert!(!findings.iter().any(|finding| {
        finding.file.starts_with("web/app/")
            && matches!(
                finding.export_name.as_str(),
                "runtime" | "alt" | "size" | "contentType"
            )
    }));
}

#[test]
fn disabled_config_skips_rule() {
    assert!(findings("unique-exports-config-disabled").is_empty());
}

#[test]
fn explicit_tsconfig_resolves_path_aliases() {
    let root = fixture("unique-exports-tsconfig-paths");
    let findings = analyze_project(&root, None, Some(&root.join("tsconfig.json"))).unwrap();
    assert!(findings.is_empty());
}

#[test]
fn relative_explicit_tsconfig_resolves_from_project_root() {
    let root = fixture("unique-exports-tsconfig-paths");
    let findings = analyze_project(&root, None, Some(Path::new("tsconfig.json"))).unwrap();

    assert!(findings.is_empty());
}

#[test]
fn nearest_tsconfig_is_discovered_and_explicit_errors_are_reported() {
    let root = fixture("unique-exports-tsconfig-paths");
    let findings = analyze_project(&root, None, None).unwrap();
    assert!(findings.is_empty());
    assert!(analyze_project(&root, None, Some(&root.join("missing-tsconfig.json"))).is_err());
}

#[test]
fn covers_reexport_resolution_edge_cases() {
    let findings = findings("unique-exports-edge-cases");
    let names = finding_names(&findings);
    assert!(!names.contains(&("Direct".to_string(), "value".to_string())));
    assert!(!names.contains(&("DirectType".to_string(), "type".to_string())));
    assert!(!names.contains(&("DefaultAlias".to_string(), "value".to_string())));
    assert!(names.contains(&("DefaultShapeAlias".to_string(), "type".to_string())));
    assert!(!names.contains(&("ChainAlias".to_string(), "type".to_string())));
    assert!(!names.contains(&("StarResolved".to_string(), "value".to_string())));
    assert!(!names.contains(&("TypeStarOnly".to_string(), "type".to_string())));
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
    let files = vec![root.join("src/direct.ts"), root.join("package.json")];

    let filtered = scan::filter_source_files(&files);
    assert_eq!(filtered.len(), 1);

    let sources = scan::collect_source_files(&root, &filtered).unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].rel, "src/direct.ts");

    assert!(scan::collect_source_files(&root, &[root.join("src/not-present.ts")]).is_err());
    let invalid_root = fixture("unique-exports-invalid-source");
    let error = scan::collect_source_files(&invalid_root, &[invalid_root.join("src/broken.ts")])
        .unwrap_err();
    assert!(format!("{error:#}").contains("extracting symbols from"));

    let disabled_invalid =
        scan::collect_source_files(&root, &[root.join("src/disabled-invalid.ts")]).unwrap();
    assert!(disabled_invalid[0].disabled);
    assert!(disabled_invalid[0].symbols.exports.is_empty());

    let lookup = scan::NextJsProjectLookup::new(&fixture("unique-exports-nextjs"), &[]);
    assert!(!lookup.contains_file(&root.join("src/direct.ts")));
    let lookup = scan::NextJsProjectLookup::new(&root, &[PathBuf::from("loose.ts")]);
    assert!(!lookup.contains_file(Path::new("loose.ts")));
    // PathBuf::from("/") has parent() == None, exercising the unwrap_or_else fallback.
    let lookup = scan::NextJsProjectLookup::new(&root, &[PathBuf::from("/")]);
    assert!(!lookup.contains_file(Path::new("/")));

    assert!(!scan::package_json_has_next_dependency(
        &fixture("unique-exports-malformed-package").join("package.json")
    ));
}

#[test]
fn defensive_helpers_ignore_missing_targets_and_non_matching_default_exports() {
    let root = fixture("unique-exports-edge-cases");
    let all_files = discover_files(&root, &[]);
    let files = scan::filter_source_files(&all_files);
    let source_files = scan::collect_source_files(&root, &files).unwrap();
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
    let mut memo = HashMap::new();
    assert!(collector::collect_file_exports(
        &root.join("src/not-present.ts"),
        &files,
        &resolver,
        &workspace,
        &mut visiting,
        &mut memo,
    )
    .is_empty());

    let mut visiting = HashSet::new();
    assert_eq!(
        collector::find_target_export_origin(
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
        collector::find_target_export_origin(
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
