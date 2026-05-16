use super::detect_has_state;
use no_mistakes_core::ast;

fn check(source: &str) -> bool {
    let path = std::path::Path::new("test.tsx");
    ast::with_program(path, source, |program, _| detect_has_state(program, source)).unwrap()
}

#[test]
fn detects_use_state() {
    assert!(check("const [x, setX] = useState(0);"));
}

#[test]
fn detects_react_use_state() {
    assert!(check("const [x, setX] = React.useState(0);"));
}

#[test]
fn detects_use_reducer() {
    assert!(check("const [state, dispatch] = useReducer(reducer, {});"));
}

#[test]
fn no_state() {
    assert!(!check("const x = someOtherHook();"));
}

#[test]
fn detects_this_set_state() {
    assert!(check("function App() { this.setState({}); }"));
}
