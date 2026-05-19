use super::{css_escape, extract_css_id_selectors};
use crate::selectors::types::PlaywrightSelector;

#[test]
fn css_escapes_handle_hex_and_non_hex() {
    assert_eq!(css_escape(r#"\20"#, 0).unwrap().0, ' ');
    assert_eq!(css_escape(r#"\:"#, 0).unwrap().0, ':');
    assert_eq!(css_escape(r#"\20 "#, 0).unwrap().0, ' ');
}

#[test]
fn extract_css_id_selectors_skips_strings_and_comments() {
    let mut found: Vec<PlaywrightSelector> = Vec::new();
    let insert = &mut |s: PlaywrightSelector| found.push(s);

    extract_css_id_selectors(r##"const a = "#real-id";"##, insert);
    assert!(
        found.is_empty(),
        "id inside double-quoted string should be skipped"
    );

    let mut found2: Vec<PlaywrightSelector> = Vec::new();
    let insert2 = &mut |s: PlaywrightSelector| found2.push(s);
    extract_css_id_selectors("const b = '#fake-id';", insert2);
    assert!(
        found2.is_empty(),
        "id inside single-quoted string should be skipped"
    );

    let mut found3: Vec<PlaywrightSelector> = Vec::new();
    let insert3 = &mut |s: PlaywrightSelector| found3.push(s);
    extract_css_id_selectors("/* #in-comment */ #out-comment", insert3);
    assert_eq!(found3.len(), 1, "id outside comment should be extracted");
    assert_eq!(found3[0].selector, "#out-comment");
}
