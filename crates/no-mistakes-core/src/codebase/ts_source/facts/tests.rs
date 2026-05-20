use super::*;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/ast-snippets/ts-source/facts")
        .join(name)
}

#[test]
fn plan_constructors_select_expected_fact_sets() {
    let imports = TsFactPlan::imports();
    assert!(imports.imports);
    assert!(!imports.symbols);

    let both = TsFactPlan::imports_and_symbols();
    assert!(both.imports);
    assert!(both.symbols);
}

#[test]
fn plan_domain_fact_detection_tracks_domain_flags() {
    assert!(!TsFactPlan::default().has_domain_facts());
    assert!(!TsFactPlan {
        imports: true,
        symbols: true,
        ..TsFactPlan::default()
    }
    .has_domain_facts());
    assert!(!TsFactPlan {
        source: true,
        ..TsFactPlan::default()
    }
    .has_domain_facts());

    for plan in [
        TsFactPlan {
            route_refs: true,
            ..TsFactPlan::default()
        },
        TsFactPlan {
            backend_routes: true,
            ..TsFactPlan::default()
        },
        TsFactPlan {
            queue_usage: true,
            ..TsFactPlan::default()
        },
        TsFactPlan {
            queue_factory: true,
            ..TsFactPlan::default()
        },
        TsFactPlan {
            http_calls: true,
            ..TsFactPlan::default()
        },
        TsFactPlan {
            process_spawns: true,
            ..TsFactPlan::default()
        },
    ] {
        assert!(plan.has_domain_facts());
    }
}

#[test]
#[should_panic(expected = "domain fact plans require collect_ts_facts_with_context")]
fn collect_ts_facts_rejects_context_required_domain_plans() {
    let ts = fixture("imports.ts");
    let _facts = collect_ts_facts(
        std::slice::from_ref(&ts),
        TsFactPlan {
            http_calls: true,
            ..TsFactPlan::default()
        },
    );
}

#[test]
fn collect_ts_facts_can_include_source_without_domain_context() {
    let ts = fixture("imports.ts");
    let facts = collect_ts_facts(
        std::slice::from_ref(&ts),
        TsFactPlan {
            source: true,
            ..TsFactPlan::default()
        },
    );

    assert!(facts[&ts]
        .source
        .as_deref()
        .unwrap_or("")
        .contains("import"));
}

#[test]
fn plan_empty_detection_tracks_all_flags() {
    assert!(TsFactPlan::default().is_empty());

    for plan in [
        TsFactPlan {
            imports: true,
            ..TsFactPlan::default()
        },
        TsFactPlan {
            symbols: true,
            ..TsFactPlan::default()
        },
        TsFactPlan {
            source: true,
            ..TsFactPlan::default()
        },
        TsFactPlan {
            route_refs: true,
            ..TsFactPlan::default()
        },
        TsFactPlan {
            backend_routes: true,
            ..TsFactPlan::default()
        },
        TsFactPlan {
            queue_usage: true,
            ..TsFactPlan::default()
        },
        TsFactPlan {
            queue_factory: true,
            ..TsFactPlan::default()
        },
        TsFactPlan {
            http_calls: true,
            ..TsFactPlan::default()
        },
        TsFactPlan {
            process_spawns: true,
            ..TsFactPlan::default()
        },
    ] {
        assert!(!plan.is_empty());
    }
}

#[test]
fn unscoped_domain_fact_context_does_not_collect_config_scoped_facts() {
    let ts = fixture("imports.ts");
    let context = TsFactContext::new(ts.parent().unwrap());
    let facts = collect_ts_facts_with_context(
        std::slice::from_ref(&ts),
        TsFactPlan {
            backend_routes: true,
            queue_factory: true,
            ..TsFactPlan::default()
        },
        &context,
    );
    let file_facts = &facts[&ts];

    assert!(file_facts.backend_routes.is_empty());
    assert!(file_facts.queue_create_line.is_none());
    assert!(file_facts.queue_name.is_none());
}

#[test]
fn queue_factory_context_requires_specifier_and_function_even_when_glob_matches() {
    let ts = fixture("imports.ts");
    let mut builder = globset::GlobSetBuilder::new();
    builder.add(globset::Glob::new("*.ts").unwrap());
    let mut context = TsFactContext::new(ts.parent().unwrap());
    context.queue_factory_glob = Some(builder.build().unwrap());
    let facts = collect_ts_facts_with_context(
        std::slice::from_ref(&ts),
        TsFactPlan {
            queue_factory: true,
            ..TsFactPlan::default()
        },
        &context,
    );

    assert!(facts[&ts].queue_create_line.is_none());
    assert!(facts[&ts].queue_name.is_none());
}

#[test]
fn collect_ts_facts_skips_non_indexable_and_unreadable_files() {
    let ts = fixture("imports.ts");
    let txt = fixture("plain.txt");
    let missing = fixture("missing.ts");
    let facts = collect_ts_facts(&[ts.clone(), txt, missing], TsFactPlan::imports());

    assert_eq!(facts.len(), 1);
    assert_eq!(facts[&ts].imports.len(), 1);
    assert!(facts[&ts].symbols.is_none());
}

#[test]
fn collect_ts_facts_uses_tsx_parser_and_symbols_when_requested() {
    let tsx = fixture("component.tsx");
    let facts = collect_ts_facts(
        std::slice::from_ref(&tsx),
        TsFactPlan::imports_and_symbols(),
    );

    assert_eq!(facts[&tsx].imports.len(), 1);
    assert!(facts[&tsx].symbols.is_some());
}

#[test]
fn collect_ts_facts_can_skip_import_collection() {
    let ts = fixture("imports.ts");
    let facts = collect_ts_facts(
        std::slice::from_ref(&ts),
        TsFactPlan {
            imports: false,
            symbols: false,
            ..TsFactPlan::default()
        },
    );

    assert!(facts[&ts].imports.is_empty());
    assert!(facts[&ts].symbols.is_none());
}

#[test]
fn collect_file_facts_falls_back_to_ts_source_type_for_unknown_extension() {
    let unknown = fixture("unknown-extension.source");
    let facts = collect_file_facts(&unknown, TsFactPlan::imports(), &TsFactContext::default())
        .expect("unknown extension fixture should still parse as TypeScript");

    assert_eq!(facts.imports.len(), 1);
}
