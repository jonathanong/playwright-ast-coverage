use super::*;

#[test]
fn extracts_inline_link() {
    let links = extract_links("[foo](./foo.mts)");
    assert_eq!(links, vec!["./foo.mts"]);
}

#[test]
fn extracts_multiple_links() {
    let md = "See [a](./a.mts) and [b](./b.mts).";
    let links = extract_links(md);
    assert_eq!(links.len(), 2);
    assert!(links.contains(&"./a.mts".to_string()));
    assert!(links.contains(&"./b.mts".to_string()));
}

#[test]
fn extracts_image_link() {
    let md = "![diagram](./arch.png)";
    let links = extract_links(md);
    assert_eq!(links, vec!["./arch.png"]);
}

#[test]
fn extracts_reference_style_link() {
    let md = "See [API][api-link].\n\n[api-link]: ./api.mts";
    let links = extract_links(md);
    assert!(links.contains(&"./api.mts".to_string()));
}

#[test]
fn empty_source_returns_empty() {
    assert!(extract_links("").is_empty());
}

#[test]
fn no_links_returns_empty() {
    assert!(extract_links("Just text here.").is_empty());
}

#[test]
fn is_external_http() {
    assert!(is_external("http://example.com"));
    assert!(is_external("https://example.com"));
}

#[test]
fn is_external_mailto() {
    assert!(is_external("mailto:user@example.com"));
}

#[test]
fn is_external_double_slash() {
    assert!(is_external("//cdn.example.com/script.js"));
}

#[test]
fn is_external_anchor() {
    assert!(is_external("#section"));
}

#[test]
fn is_external_query() {
    assert!(is_external("?search=foo"));
}

#[test]
fn not_external_relative() {
    assert!(!is_external("./foo.mts"));
    assert!(!is_external("../bar.ts"));
    assert!(!is_external("packages/api/src/index.mts"));
}
