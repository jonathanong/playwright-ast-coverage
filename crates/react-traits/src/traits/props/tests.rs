use super::detect_props;
use no_mistakes_core::ast;

fn check(source: &str) -> (bool, bool) {
    let path = std::path::Path::new("test.tsx");
    ast::with_program(path, source, |program, _| detect_props(program, source)).unwrap()
}

#[test]
fn no_props_no_passes() {
    let (has_props, passes_props) = check("export default function App() { return <div/>; }");
    assert!(!has_props);
    assert!(!passes_props);
}

#[test]
fn has_props_from_params() {
    let (has_props, _) = check("export default function App({ name }) { return <div/>; }");
    assert!(has_props);
}

#[test]
fn passes_props_to_child() {
    let (_, passes_props) = check("export default function App() { return <Child name=\"x\" />; }");
    assert!(passes_props);
}
