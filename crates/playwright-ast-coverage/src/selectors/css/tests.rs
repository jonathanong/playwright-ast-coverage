use super::css_escape;

#[test]
fn css_escapes_handle_hex_and_non_hex() {
    assert_eq!(css_escape(r#"\20"#, 0).unwrap().0, ' ');
    assert_eq!(css_escape(r#"\:"#, 0).unwrap().0, ':');
}
