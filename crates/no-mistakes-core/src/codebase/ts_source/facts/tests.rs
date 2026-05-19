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
