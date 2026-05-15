use super::*;
use std::fs;
use tempfile::tempdir;

#[derive(Default, serde::Deserialize)]
struct TestConfig {
    name: String,
}

#[test]
fn test_load_config_yaml() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("test.yaml");
    fs::write(&config_path, "name: hello\n").unwrap();

    let config: TestConfig = load_config(dir.path(), None, &["test"]).unwrap();
    assert_eq!(config.name, "hello");
}

#[test]
fn test_load_config_json() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("test.json");
    fs::write(&config_path, "{\"name\": \"world\"}").unwrap();

    let config: TestConfig = load_config(dir.path(), None, &["test"]).unwrap();
    assert_eq!(config.name, "world");
}

#[test]
fn test_load_config_priority() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("first.yaml"), "name: first\n").unwrap();
    fs::write(dir.path().join("second.yaml"), "name: second\n").unwrap();

    let config: TestConfig = load_config(dir.path(), None, &["first", "second"]).unwrap();
    assert_eq!(config.name, "first");
}

#[test]
fn test_load_config_missing_returns_default() {
    let dir = tempdir().unwrap();
    let config: TestConfig = load_config(dir.path(), None, &["missing"]).unwrap();
    assert_eq!(config.name, "");
}

#[test]
fn test_load_config_explicit() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("custom.yaml");
    fs::write(&config_path, "name: custom\n").unwrap();

    let config: TestConfig =
        load_config(dir.path(), Some(Path::new("custom.yaml")), &["test"]).unwrap();
    assert_eq!(config.name, "custom");
}

#[test]
fn test_load_config_multiple_error() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("test.yaml"), "name: a\n").unwrap();
    fs::write(dir.path().join("test.json"), "{\"name\": \"b\"}").unwrap();

    let err = load_config::<TestConfig>(dir.path(), None, &["test"])
        .err()
        .unwrap();
    assert!(err.to_string().contains("multiple config files found"));
}

#[test]
fn test_parse_config_jsonc() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("test.jsonc");
    fs::write(&config_path, "{\n  // comment\n  \"name\": \"jsonc\"\n}").unwrap();

    let config: TestConfig = load_config(dir.path(), None, &["test"]).unwrap();
    assert_eq!(config.name, "jsonc");
}

#[test]
fn test_resolve_absolute() {
    let root = Path::new("/root");
    let path = Path::new("/abs/path");
    assert_eq!(resolve(root, path), path.to_path_buf());
}

#[test]
fn test_load_config_explicit_missing() {
    let dir = tempdir().unwrap();
    let err = load_config::<TestConfig>(dir.path(), Some(Path::new("missing.yaml")), &["test"])
        .err()
        .unwrap();
    assert!(err.to_string().contains("config file does not exist"));
}

#[test]
fn test_load_config_read_error() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.yaml");
    fs::create_dir(&path).unwrap(); // Dir instead of file will cause read error

    let err = load_config::<TestConfig>(dir.path(), None, &["test"])
        .err()
        .unwrap();
    assert!(err.to_string().contains("directory") || err.to_string().contains("failed"));
}

#[test]
fn test_parse_config_jsonc_error() {
    let err = parse_config::<TestConfig>("{\n  \"name\": \n}", Path::new("test.jsonc"))
        .err()
        .unwrap();
    assert!(err.to_string().contains("Unexpected close brace"));

    let err = parse_config::<TestConfig>("", Path::new("test.jsonc"))
        .err()
        .unwrap();
    assert!(err.to_string().contains("invalid"));
}

#[test]
fn test_parse_config_unsupported_extension() {
    let err = parse_config::<TestConfig>("", Path::new("test.toml"))
        .err()
        .unwrap();
    assert!(err
        .to_string()
        .contains("unsupported config file extension"));
}

#[test]
fn test_parse_config_no_extension() {
    let err = parse_config::<TestConfig>("", Path::new("test"))
        .err()
        .unwrap();
    assert!(err
        .to_string()
        .contains("unsupported config file without extension"));
}
