use crate::selectors::types::TemplatePattern;

#[test]
fn malformed_template_is_treated_as_literal_pattern() {
    let pattern = TemplatePattern::new("user-${id").unwrap();
    assert!(pattern.matches_exact("user-${id"));
    assert_eq!(pattern.sample(), "user-${id");
}

#[test]
fn template_without_static_parts_is_unsupported() {
    assert!(TemplatePattern::new("${id}").is_none());
}

#[test]
fn template_exact_matching_rejects_non_matching_values() {
    let pattern = TemplatePattern::new("user-${id}-button").unwrap();
    assert!(!pattern.matches_exact("admin-1-button"));
    assert!(!pattern.matches_exact("user-1-link"));
    assert!(!pattern.matches_exact("user-1"));
    assert!(!pattern.matches_exact("user-button"));
}

#[test]
fn empty_internal_template_pattern_does_not_match() {
    let pattern = TemplatePattern {
        raw: String::new(),
        parts: vec![String::new()],
        starts_static: false,
        ends_static: false,
    };
    assert!(!pattern.matches_exact("anything"));
}
