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
        collect_jsx_children(program, &table)
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
