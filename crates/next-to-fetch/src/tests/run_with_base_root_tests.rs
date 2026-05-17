use crate::cli::Cli;
use crate::pipeline::run::run_with_base_root;
use no_mistakes_core::cli::Format;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_run_with_base_root_errors_when_route_target_matcher_fails() {
    let root = tempdir().unwrap();
    fs::create_dir(root.path().join("app")).unwrap();
    fs::write(root.path().join("app/page.tsx"), "export const = invalid;").unwrap();
    fs::write(root.path().join("app/target.ts"), "fetch('/api/target');").unwrap();

    let cli = Cli {
        root: PathBuf::from("."),
        config: None,
        format: Format::Human,
        json: false,
        targets: vec!["app/target.ts".to_string()],
    };

    let err = run_with_base_root(root.path(), &cli).err().unwrap();
    assert!(err.to_string().contains("parse") || err.to_string().contains("expected"));
}

#[test]
fn test_run_with_base_root_errors_when_layout_target_matcher_fails() {
    let root = tempdir().unwrap();
    fs::create_dir(root.path().join("app")).unwrap();
    fs::write(root.path().join("app/page.tsx"), "export {}").unwrap();
    fs::write(
        root.path().join("app/layout.tsx"),
        "import { helper } from './target'; const = invalid;",
    )
    .unwrap();
    fs::write(root.path().join("app/target.ts"), "export {}").unwrap();

    let cli = Cli {
        root: PathBuf::from("."),
        config: None,
        format: Format::Human,
        json: false,
        targets: vec!["app/target.ts".to_string()],
    };

    let err = run_with_base_root(root.path(), &cli).err().unwrap();
    assert!(err.to_string().contains("parse") || err.to_string().contains("expected"));
}

#[test]
fn test_run_with_base_root_errors_when_route_analysis_fails() {
    let root = tempdir().unwrap();
    fs::create_dir(root.path().join("app")).unwrap();
    fs::write(root.path().join("app/page.tsx"), "export const = invalid;").unwrap();

    let cli = Cli {
        root: PathBuf::from("."),
        config: None,
        format: Format::Human,
        json: false,
        targets: vec![],
    };

    let err = run_with_base_root(root.path(), &cli).err().unwrap();
    assert!(err.to_string().contains("parse") || err.to_string().contains("expected"));
}

#[test]
fn test_run_with_base_root_errors_when_layout_analysis_fails() {
    let root = tempdir().unwrap();
    fs::create_dir(root.path().join("app")).unwrap();
    fs::write(root.path().join("app/page.tsx"), "fetch('/api/page');").unwrap();
    fs::write(
        root.path().join("app/layout.tsx"),
        "export const = invalid;",
    )
    .unwrap();

    let cli = Cli {
        root: PathBuf::from("."),
        config: None,
        format: Format::Human,
        json: false,
        targets: vec![],
    };

    let err = run_with_base_root(root.path(), &cli).err().unwrap();
    assert!(err.to_string().contains("parse") || err.to_string().contains("expected"));
}

#[test]
fn test_run_with_base_root_errors_when_target_is_unmatched() {
    let root = tempdir().unwrap();
    fs::create_dir(root.path().join("app")).unwrap();
    fs::write(root.path().join("app/page.tsx"), "fetch('/api/page');").unwrap();

    let cli = Cli {
        root: PathBuf::from("."),
        config: None,
        format: Format::Human,
        json: false,
        targets: vec!["app/missing.ts".to_string()],
    };

    let err = run_with_base_root(root.path(), &cli).err().unwrap();
    let message = err.to_string();
    assert!(message.contains("Error: targets not found"));
    assert!(message.contains("app/missing.ts"));
}

#[test]
fn test_run_with_base_root_sorts_duplicates_and_unsupported_entries() {
    let root = tempdir().unwrap();
    fs::create_dir_all(root.path().join("app/about")).unwrap();
    fs::write(
        root.path().join("app/page.tsx"),
        "
            fetch(`/api/${route}/users`);
            fetch('/api/duplicate');
            fetch('/api/duplicate');
            ",
    )
    .unwrap();
    fs::write(
        root.path().join("app/about/page.tsx"),
        "
            fetch(`/api/${about}/users`);
            fetch('/api/other');
            fetch('/api/other');
            ",
    )
    .unwrap();

    let cli = Cli {
        root: PathBuf::from("."),
        config: None,
        format: Format::Human,
        json: false,
        targets: vec![],
    };
    let report = run_with_base_root(root.path(), &cli).unwrap();

    assert_eq!(report.unsupported.len(), 2);
    assert_eq!(report.unsupported[0].route, "/");
    assert!(report.unsupported[0].file.ends_with("app/page.tsx"));
    assert_eq!(report.unsupported[1].route, "/about");
    assert!(report.unsupported[1].file.ends_with("app/about/page.tsx"));

    assert_eq!(report.unsupported[0].reason, "dynamic-path");
    assert_eq!(report.unsupported[1].reason, "dynamic-path");

    assert_eq!(report.duplicates.len(), 2);
    assert_eq!(report.duplicates[0].key, "GET /api/duplicate server rsc");
    assert_eq!(report.duplicates[0].count, 2);
    assert_eq!(report.duplicates[1].key, "GET /api/other server rsc");
    assert_eq!(report.duplicates[1].count, 2);
}
