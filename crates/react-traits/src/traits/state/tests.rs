use super::detect_has_state;
use no_mistakes_core::ast;

fn check(source: &str) -> bool {
    let path = std::path::Path::new("test.tsx");
    let span = oxc_span::Span::new(0, source.len() as u32);
    ast::with_program(path, source, |program, _| detect_has_state(program, span)).unwrap()
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

#[test]
fn detects_this_state_access() {
    // `this.state` (not setState) — exercises the `prop == "state"` branch (line 47).
    assert!(check("class App { render() { return this.state; } }"));
}

#[test]
fn no_state_computed_callee() {
    // obj[key]() — computed callee hits _ => None branch.
    assert!(!check("const x = obj[key]();"));
}

#[test]
fn hook_outside_span_not_detected() {
    // Span that covers nothing — visit_call_expression returns early (line 17).
    let source = "const [x, setX] = useState(0);";
    let path = std::path::Path::new("test.tsx");
    let result = no_mistakes_core::ast::with_program(path, source, |program, _| {
        super::detect_has_state(program, oxc_span::Span::new(0, 0))
    })
    .unwrap();
    assert!(!result);
}

#[test]
fn this_other_prop_not_state() {
    // `this.render` is a StaticMemberExpression on `this` but the property is
    // neither "state" nor "setState" — exercises the false branch of line 47-49.
    assert!(!check("class App { foo() { return this.render; } }"));
}

#[test]
fn static_member_outside_span_not_detected() {
    // Span that covers nothing — visit_static_member_expression returns early (line 43).
    // Use a standalone member access (not a call) so visit_call_expression doesn't short-circuit.
    let source = "function App() { const s = this.state; }";
    let path = std::path::Path::new("test.tsx");
    let result = no_mistakes_core::ast::with_program(path, source, |program, _| {
        super::detect_has_state(program, oxc_span::Span::new(0, 0))
    })
    .unwrap();
    assert!(!result);
}
