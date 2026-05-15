use super::*;
use regex::Regex;

#[test]
fn identifier_reassignment_uses_identifier_boundaries_and_assignment_operator() {
    assert!(has_identifier_reassignment("dataPw = makeId();", "dataPw"));
    assert!(has_identifier_reassignment(
        "data$Pw = makeId();",
        "data$Pw"
    ));
    assert!(!has_identifier_reassignment("xdataPw = 1;", "dataPw"));
    assert!(!has_identifier_reassignment("dataPwx = 1;", "dataPw"));
    assert!(has_identifier_reassignment("dataPw = 1;", "dataPw"));
    assert!(has_identifier_reassignment("dataPw += 1;", "dataPw"));
    assert!(has_identifier_reassignment("dataPw++", "dataPw"));
    assert!(has_identifier_reassignment("++dataPw", "dataPw"));
    assert!(has_identifier_reassignment("dataPw += '-x';", "dataPw"));
    assert!(has_identifier_reassignment(
        "dataPw ??= makeId();",
        "dataPw"
    ));
    assert!(has_identifier_reassignment("dataPw++;", "dataPw"));
    assert!(has_identifier_reassignment("--dataPw;", "dataPw"));
    assert!(!has_identifier_reassignment("dataPw === 'save';", "dataPw"));
    assert!(!has_identifier_reassignment("dataPw == 'save';", "dataPw"));
    assert!(!has_identifier_reassignment(
        "// dataPw = makeId();\nconst message = \"dataPw += '-x';\";",
        "dataPw"
    ));
    assert!(!has_identifier_reassignment("userid = makeId();", "id"));
    assert!(!has_identifier_reassignment("id => id", "id"));
}

#[test]
fn enclosing_shadow_binding_requires_an_open_block() {
    let binding = Regex::new(r"\bfunction\b[^(]*\([^)]*\bdataPw\b").unwrap();

    assert!(has_enclosing_shadow_binding(
        "function Inner(dataPw) { return <a data-pw={",
        &binding
    ));
    assert!(has_enclosing_shadow_binding(
        "function Inner(dataPw) { if (ready) { dataPw; } return <a data-pw={",
        &binding
    ));
    assert!(!has_enclosing_shadow_binding(
        "function Inner(dataPw)",
        &binding
    ));
    assert!(!has_enclosing_shadow_binding(
        "function Inner(dataPw); return <a data-pw={",
        &binding
    ));
    assert!(!has_enclosing_shadow_binding(
        "function Inner(dataPw) { return dataPw; } return <a data-pw={",
        &binding
    ));
}
