use crate::playwright_urls::callee::is_candidate_url;
use crate::playwright_urls::normalize::glob_url_sample;
use crate::playwright_urls::regex_sample::regex_path_sample;
use crate::playwright_urls::statics::{collect_static_zero_arg_paths, source_offset_is_code};

#[test]
fn regex_path_sample_edge_cases() {
    assert_eq!(
        regex_path_sample(r#"^\/orders"#),
        Some("/orders".to_string())
    );
    assert_eq!(
        regex_path_sample(r#"^\/orders\/$"#),
        Some("/orders/".to_string())
    );
    assert_eq!(regex_path_sample("^\\/orders\\/\\\""), None);
    assert_eq!(regex_path_sample(r#"^\/"#), Some("/".to_string()));
    assert_eq!(regex_path_sample(r#"^\/a\xb"#), None);
    assert_eq!(regex_path_sample(r#"^"#), None);
    assert_eq!(regex_path_sample(r#"^\"#), None);
}

#[test]
fn candidate_url_checks_schemes() {
    assert!(is_candidate_url("/path"));
    assert!(is_candidate_url("http://localhost"));
    assert!(is_candidate_url("https://localhost"));
    assert!(!is_candidate_url("localhost"));
    assert!(!is_candidate_url("ws://localhost"));
}

#[test]
fn glob_url_sample_handles_edge_cases() {
    assert_eq!(glob_url_sample("/${path}"), None);
    assert_eq!(
        glob_url_sample("host.com/*/path"),
        Some("/x/path".to_string())
    );
    assert_eq!(glob_url_sample("*/path"), Some("/path".to_string()));
}

#[test]
fn samples_simple_url_regex_literals() {
    assert_eq!(
        regex_path_sample(r#"^/orders/[a-z\]]+/.{2,4}$"#),
        Some("/orders/x/x".to_string())
    );
    assert_eq!(
        regex_path_sample(r#"^\/orders\/.*?$"#),
        Some("/orders/x".to_string())
    );
    assert_eq!(
        regex_path_sample(r#"^\/orders\/\%bad$"#),
        Some("/orders/%bad".to_string())
    );
    assert_eq!(regex_path_sample(r#"^\/orders\/\#$"#), None);
    assert_eq!(
        regex_path_sample(r#"^\s/orders$"#),
        Some("/orders".to_string())
    );
    assert_eq!(
        regex_path_sample(r#"^\/orders\/\"#),
        Some("/orders/".to_string())
    );
    assert_eq!(
        regex_path_sample(r#"^https:\/\/example.com\/orders$"#),
        Some("https://example.com/orders".to_string())
    );
    assert_eq!(
        regex_path_sample(r#"^https:\/\/example\.com\/orders$"#),
        Some("https://example.com/orders".to_string())
    );
    assert_eq!(regex_path_sample(r#"^/orders/<id>$"#), None);
    assert_eq!(regex_path_sample(r#"^/orders/(\d+)$"#), None);
    assert_eq!(regex_path_sample(r#"^\/users\/\d+$"#), None);
    assert_eq!(regex_path_sample(r#"^not-a-path$"#), None);
    assert_eq!(regex_path_sample(r#"^/$"#), Some("/".to_string()));
    assert_eq!(regex_path_sample(r#"^\/$"#), Some("/".to_string()));
}

#[test]
fn samples_glob_url_patterns() {
    assert_eq!(
        glob_url_sample("**/orders/*/details"),
        Some("/orders/x/details".to_string())
    );
    assert_eq!(
        glob_url_sample("*/orders/**"),
        Some("/orders/x".to_string())
    );
    assert_eq!(
        glob_url_sample("https://example.com/orders/**"),
        Some("https://example.com/orders/x".to_string())
    );
    assert_eq!(
        glob_url_sample("example.com/orders/**"),
        Some("/orders/x".to_string())
    );
    assert_eq!(glob_url_sample("orders/**"), Some("/orders/x".to_string()));
    assert_eq!(glob_url_sample("orders*"), Some("/ordersx".to_string()));
    assert_eq!(glob_url_sample("**/"), None);
    assert_eq!(glob_url_sample("**/${path}"), None);
    assert_eq!(glob_url_sample("/orders"), None);
}

#[test]
fn source_offset_filter_ignores_comments_and_strings() {
    let source = "'route: () => \\'/string\\'';\n/* route: () => '/block' */\n// route: () => '/line'\nconst route = () => '/real';";
    assert!(!source_offset_is_code(
        source,
        source.find("route:").unwrap()
    ));
    assert!(!source_offset_is_code(
        source,
        source.find("block").unwrap() - 14
    ));
    assert!(!source_offset_is_code(
        source,
        source.find("line").unwrap() - 14
    ));
    assert!(source_offset_is_code(
        source,
        source.rfind("route").unwrap()
    ));
    assert!(!source_offset_is_code(
        "'unterminated",
        "'unterminated".len()
    ));

    let double_quoted = r#"const x = "foo: () => \"/bar\""; const real = 1;"#;
    assert!(!source_offset_is_code(
        double_quoted,
        double_quoted.find("bar").unwrap()
    ));
    assert!(source_offset_is_code(
        double_quoted,
        double_quoted.find("real").unwrap()
    ));

    let template = "const x = `foo: () => '/bar'`; const real = 1;";
    assert!(!source_offset_is_code(
        template,
        template.find("bar").unwrap()
    ));
    assert!(source_offset_is_code(
        template,
        template.find("real").unwrap()
    ));
    assert!(!source_offset_is_code(
        "`unterminated",
        "`unterminated".len()
    ));
}

#[test]
fn collect_static_zero_arg_paths_handles_various_quote_styles() {
    let source = r#"
        const routes = {
            home: () => "/home",
            about: () => '/about',
            help: () => `/help`,
        };
    "#;
    let paths = collect_static_zero_arg_paths(source);
    assert!(paths.contains_key("home"), "double-quoted route not found");
    assert!(paths.contains_key("about"), "single-quoted route not found");
    assert!(
        paths.contains_key("help"),
        "template-literal route not found"
    );
}
