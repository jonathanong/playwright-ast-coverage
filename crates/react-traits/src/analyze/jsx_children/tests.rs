use super::collect_jsx_children;
use crate::analyze::import_table::build_import_table;
use no_mistakes_core::ast;
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/react-traits-analyze/import-table")
}

fn collect_children_names(source: &str, file: &std::path::Path) -> Vec<String> {
    ast::with_program(file, source, |program, _| {
        let table = build_import_table(file, program);
        let span = oxc_span::Span::new(0, source.len() as u32);
        collect_jsx_children(program, &table, &file.to_path_buf(), span)
            .into_iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>()
    })
    .unwrap()
}

#[test]
fn capitalized_tag_resolved_to_import() {
    let dir = fixture_dir();
    let file = dir.join("Consumer.tsx");
    let source = "import Foo from './Foo';\nexport default function App() { return <Foo />; }";
    let names = collect_children_names(source, &file);
    assert_eq!(names, vec!["default"]);
}

#[test]
fn member_expression_tag_resolved_to_import() {
    let dir = fixture_dir();
    let file = dir.join("Consumer.tsx");
    // <Foo.Bar /> — jsx_member_root returns "Foo", which is in the import table as
    // a default import. The local_name "Foo" does not contain '.', so exported_name
    // is the entry's exported_name ("default").
    let source = "import Foo from './Foo';\nexport default function App() { return <Foo.Bar />; }";
    let names = collect_children_names(source, &file);
    assert_eq!(names, vec!["default"]);
}

#[test]
fn lowercase_tag_ignored() {
    let dir = fixture_dir();
    let file = dir.join("Consumer.tsx");
    let source = "import Foo from './Foo';\nexport default function App() { return <div />; }";
    let names = collect_children_names(source, &file);
    assert!(names.is_empty());
}

#[test]
fn tag_not_in_import_table_ignored() {
    let dir = fixture_dir();
    let file = dir.join("Consumer.tsx");
    let source = "export default function App() { return <Foo />; }";
    let names = collect_children_names(source, &file);
    assert!(names.is_empty());
}

#[test]
fn nested_member_expression_tag_resolved() {
    let dir = fixture_dir();
    let file = dir.join("Consumer.tsx");
    // <Foo.Bar.Baz /> — jsx_member_root recursively finds the root "Foo"
    let source =
        "import Foo from './Foo';\nexport default function App() { return <Foo.Bar.Baz />; }";
    let names = collect_children_names(source, &file);
    assert_eq!(names, vec!["default"]);
}

#[test]
fn this_expression_member_tag_ignored() {
    // <this.Foo /> — jsx_member_root returns "" for ThisExpression.
    // "" is not in the import table, so the child is silently ignored.
    let dir = fixture_dir();
    let file = dir.join("Consumer.tsx");
    let source = "export default class App { render() { return <this.Foo />; } }";
    let names = collect_children_names(source, &file);
    assert!(names.is_empty());
}

#[test]
fn children_outside_span_excluded() {
    let dir = fixture_dir();
    let file = dir.join("Consumer.tsx");
    // Span(0,0) covers nothing — all JSX elements are outside it.
    let source = "import Foo from './Foo';\nexport default function App() { return <Foo />; }";
    let names = ast::with_program(&file, source, |program, _| {
        let table = build_import_table(&file, program);
        collect_jsx_children(
            program,
            &table,
            &file.to_path_buf(),
            oxc_span::Span::new(0, 0),
        )
        .into_iter()
        .map(|(_, name)| name)
        .collect::<Vec<_>>()
    })
    .unwrap();
    assert!(names.is_empty(), "elements outside span should be excluded");
}

#[test]
fn same_file_component_resolved() {
    // Child defined in the same file (no import) — resolves to file itself (Chpey)
    let dir = fixture_dir();
    let file = dir.join("Consumer.tsx");
    let source =
        "export const Child = () => <div/>;\nexport default function Parent() { return <Child/>; }";
    let names = ast::with_program(&file, source, |program, _| {
        let table = build_import_table(&file, program);
        let span = oxc_span::Span::new(0, source.len() as u32);
        collect_jsx_children(program, &table, &file.to_path_buf(), span)
            .into_iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>()
    })
    .unwrap();
    assert_eq!(names, vec!["Child"]);
}

#[test]
fn same_file_aliased_export_resolved_by_local_name() {
    // `export { Foo as Bar }` — JSX uses `<Foo/>` (local name), should resolve to "Bar"
    let dir = fixture_dir();
    let file = dir.join("Consumer.tsx");
    let source =
        "const Foo = () => <div/>;\nexport { Foo as Bar };\nexport default function P() { return <Foo/>; }";
    let names = ast::with_program(&file, source, |program, _| {
        let table = build_import_table(&file, program);
        let span = oxc_span::Span::new(0, source.len() as u32);
        collect_jsx_children(program, &table, &file.to_path_buf(), span)
            .into_iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>()
    })
    .unwrap();
    assert_eq!(names, vec!["Bar"]);
}

#[test]
fn same_file_default_export_identifier_not_mapped_to_other_components() {
    // `const Foo = () => ...; const Page = () => ...; export default Page;`
    // Only "Page" should map to "default"; "Foo" should NOT be in the local_components map.
    let dir = fixture_dir();
    let file = dir.join("Consumer.tsx");
    let source = "const Foo = () => <div/>;\nconst Page = () => <Foo/>;\nexport default Page;";
    let names = ast::with_program(&file, source, |program, _| {
        let table = build_import_table(&file, program);
        let span = oxc_span::Span::new(0, source.len() as u32);
        collect_jsx_children(program, &table, &file.to_path_buf(), span)
            .into_iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>()
    })
    .unwrap();
    // <Foo/> is rendered inside Page; Foo is not in the export map, so it's not resolved
    assert!(
        names.is_empty(),
        "Foo should not be mapped as a child since it is not exported"
    );
}

#[test]
fn namespace_import_member_resolved_to_suffix() {
    // `import * as Foo from './Foo'; <Foo.Bar/>` — namespace wildcard import with member
    // expression; exercises the `suffix.clone()` branch (line 66)
    let dir = fixture_dir();
    let file = dir.join("Consumer.tsx");
    let source =
        "import * as Foo from './Foo';\nexport default function App() { return <Foo.Bar/>; }";
    let names = collect_children_names(source, &file);
    assert_eq!(names, vec!["Bar"]);
}

#[test]
fn destructured_export_var_not_in_local_components() {
    // `export const [Foo] = []` — ArrayPattern binding; hits line 124 in collect_local_components
    let dir = fixture_dir();
    let file = dir.join("Consumer.tsx");
    let source = "export const [Foo] = [];\nexport default function App() { return <Foo/>; }";
    let names = collect_children_names(source, &file);
    assert!(names.is_empty());
}

#[test]
fn class_export_not_in_local_components() {
    // `export class Foo {}` without superclass — guard fails, not mapped
    let dir = fixture_dir();
    let file = dir.join("Consumer.tsx");
    let source = "export class Foo {};\nexport default function App() { return <Foo/>; }";
    let names = collect_children_names(source, &file);
    assert!(names.is_empty());
}

#[test]
fn class_export_extends_component_in_local_components() {
    // `export class Foo extends Component {}` — is_class_component passes; Foo is mapped
    let dir = fixture_dir();
    let file = dir.join("Consumer.tsx");
    let source =
        "export class Foo extends Component {};\nexport default function App() { return <Foo/>; }";
    let names = collect_children_names(source, &file);
    assert_eq!(names, vec!["Foo"]);
}

#[test]
fn default_export_memo_wrapping_maps_identifier() {
    // `export default memo(Page)` — Page is mapped to "default"
    let dir = fixture_dir();
    let file = dir.join("Consumer.tsx");
    let source = "const Page = () => <div/>;\nexport default memo(Page);\nexport function Parent() { return <Page/>; }";
    let names = collect_children_names(source, &file);
    assert_eq!(names, vec!["default"]);
}

#[test]
fn default_export_call_no_args_no_mapping() {
    // `export default memo()` — no first arg; CallExpression branch with no identifier
    let dir = fixture_dir();
    let file = dir.join("Consumer.tsx");
    let source = "export default memo();\nexport function App() { return <div/>; }";
    let names = collect_children_names(source, &file);
    assert!(names.is_empty());
}
