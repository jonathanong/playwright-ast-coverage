use super::types::{FileConfig, RootConfig};

#[test]
fn into_file_config_no_react_traits_section() {
    let root = RootConfig {
        legacy: FileConfig {
            frontend_root: Some("app".into()),
            assert_no_fetch: None,
        },
        react_traits: None,
    };
    let fc = root.into_file_config();
    assert_eq!(fc.frontend_root.as_deref(), Some("app"));
    assert!(fc.assert_no_fetch.is_none());
}

#[test]
fn into_file_config_partial_override_only_assert_no_fetch() {
    let root = RootConfig {
        legacy: FileConfig {
            frontend_root: Some("app".into()),
            assert_no_fetch: None,
        },
        react_traits: Some(FileConfig {
            frontend_root: None,
            assert_no_fetch: Some(true),
        }),
    };
    let fc = root.into_file_config();
    assert_eq!(fc.frontend_root.as_deref(), Some("app"));
    assert_eq!(fc.assert_no_fetch, Some(true));
}

#[test]
fn into_file_config_partial_override_only_frontend_root() {
    let root = RootConfig {
        legacy: FileConfig {
            frontend_root: None,
            assert_no_fetch: Some(false),
        },
        react_traits: Some(FileConfig {
            frontend_root: Some("src/app".into()),
            assert_no_fetch: None,
        }),
    };
    let fc = root.into_file_config();
    assert_eq!(fc.frontend_root.as_deref(), Some("src/app"));
    assert_eq!(fc.assert_no_fetch, Some(false));
}

#[test]
fn into_file_config_full_override() {
    let root = RootConfig {
        legacy: FileConfig {
            frontend_root: Some("app".into()),
            assert_no_fetch: None,
        },
        react_traits: Some(FileConfig {
            frontend_root: Some("src/app".into()),
            assert_no_fetch: Some(true),
        }),
    };
    let fc = root.into_file_config();
    assert_eq!(fc.frontend_root.as_deref(), Some("src/app"));
    assert_eq!(fc.assert_no_fetch, Some(true));
}
