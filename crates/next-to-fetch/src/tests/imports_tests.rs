use crate::analyze::import_shape::is_runtime_import;
use crate::analyze::imports::{
    collect_identifier_references, collect_imports, collect_imports_from_program,
    collect_runtime_imports_from_program,
};
use anyhow::Result;
use no_mistakes_core::ast;
use oxc_ast::ast::Statement;
use std::collections::HashMap;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_collect_imports_reuses_cached_imports() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("pkg")).unwrap();
    fs::write(dir.path().join("pkg/side-effect.ts"), "").unwrap();
    fs::write(dir.path().join("pkg/types.ts"), "").unwrap();
    let file = dir.path().join("pkg/index.ts");
    fs::write(
        &file,
        "
                import './side-effect';
                import type { Foo } from './types';
            ",
    )
    .unwrap();

    let mut import_cache = HashMap::new();
    let first = collect_imports(&file, &mut import_cache).unwrap();
    let second = collect_imports(&file, &mut import_cache).unwrap();
    assert_eq!(first, second);
}

#[test]
fn test_collect_imports_from_program_reuses_cached_value() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("main.ts");
    fs::write(file.parent().unwrap().join("side-effect.ts"), "").unwrap();
    fs::write(
        &file,
        "
            import './side-effect';
            ",
    )
    .unwrap();

    let abs_path = file.canonicalize().unwrap();
    let source = std::fs::read_to_string(&abs_path).unwrap();
    let mut import_cache = HashMap::new();
    let mut from_source = false;
    let _ = ast::with_program(&abs_path, &source, |program, _source| -> Result<()> {
        let first = collect_imports_from_program(&abs_path, program, &mut import_cache).unwrap();
        let second = collect_imports_from_program(&abs_path, program, &mut import_cache).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 1);
        from_source = !first.is_empty();
        Ok(())
    })
    .unwrap();
    assert!(from_source);
}

#[test]
fn test_collect_imports_filters_runtime_and_type_only_imports_exports() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("pkg")).unwrap();

    fs::write(dir.path().join("pkg/side-effect.ts"), "").unwrap();
    fs::write(dir.path().join("pkg/runtime.ts"), "").unwrap();
    fs::write(dir.path().join("pkg/runtime-all.ts"), "").unwrap();
    fs::write(dir.path().join("pkg/types.ts"), "").unwrap();

    let file = dir.path().join("pkg/index.ts");
    fs::write(
        &file,
        "
            import './side-effect';
            import type { Foo } from './types';
            export type { Foo } from './types';
            export type * from './types';
            export { runtimeExport } from './runtime';
            export * from './runtime-all';
            ",
    )
    .unwrap();

    let mut import_cache = HashMap::new();
    let imports = collect_imports(&file, &mut import_cache).unwrap();
    assert_eq!(imports.len(), 3);
    assert!(imports.iter().any(|path| path.ends_with("side-effect.ts")));
    assert!(imports.iter().any(|path| path.ends_with("runtime.ts")));
    assert!(imports.iter().any(|path| path.ends_with("runtime-all.ts")));
    assert!(!imports.iter().any(|path| path.ends_with("types.ts")));
}

#[test]
fn test_collect_runtime_imports_from_program_follows_used_runtime_imports_only() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "
            import './side-effect';
            import type { Foo } from './types';
            import { used, unused } from './named';
            import defaultImport from './default';
            import * as namespaceImport from './namespace';
            import { onlyUnused } from './only-unused';
            defaultImport();
            namespaceImport.helper();
            used();
            fetch('/api/entry');
        ";
    let source_type = oxc_span::SourceType::from_path(std::path::Path::new("main.ts")).unwrap();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path()).unwrap();
    fs::write(dir.path().join("side-effect.ts"), "").unwrap();
    fs::write(dir.path().join("named.ts"), "").unwrap();
    fs::write(dir.path().join("default.ts"), "").unwrap();
    fs::write(dir.path().join("namespace.ts"), "").unwrap();
    fs::write(dir.path().join("only-unused.ts"), "").unwrap();

    let main_file = dir.path().join("main.ts");
    fs::write(&main_file, source).unwrap();
    let referenced_identifiers = collect_identifier_references(&parsed.program);
    let imports =
        collect_runtime_imports_from_program(&main_file, &parsed.program, &referenced_identifiers)
            .unwrap();

    assert_eq!(imports.len(), 4);
    assert!(imports.iter().any(|path| path.ends_with("side-effect.ts")));
    assert!(imports.iter().any(|path| path.ends_with("named.ts")));
    assert!(imports.iter().any(|path| path.ends_with("default.ts")));
    assert!(imports.iter().any(|path| path.ends_with("namespace.ts")));
    assert!(!imports.iter().any(|path| path.ends_with("only-unused.ts")));
    assert!(!imports.iter().any(|path| path.ends_with("types.ts")));
}

#[test]
fn test_is_runtime_import_variants() {
    let allocator = oxc_allocator::Allocator::default();
    let source = "
            const constant = 1;
            import type { Foo } from './foo';
            import {} from './empty';
            import { type Bar } from './bar';
            import { Baz } from './baz';
            import Widget, { type Props } from './widget';
            import * as all from './all';
        ";
    let source_type = oxc_span::SourceType::from_path(std::path::Path::new("test.ts")).unwrap();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    assert!(
        parsed.errors.is_empty(),
        "import parser errors: {:?}",
        parsed.errors
    );
    let imports = parsed
        .program
        .body
        .iter()
        .filter_map(|stmt| match stmt {
            Statement::ImportDeclaration(import) => Some(import),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(imports.len(), 6);
    assert!(!is_runtime_import(imports[0]));
    assert!(is_runtime_import(imports[1]));
    assert!(!is_runtime_import(imports[2]));
    assert!(is_runtime_import(imports[3]));
    assert!(is_runtime_import(imports[4]));
    assert!(is_runtime_import(imports[5]));
}
