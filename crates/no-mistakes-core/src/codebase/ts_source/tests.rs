use super::{
    discover_files, git_visible_files, has_disable_comment, has_disable_file_comment, is_test_file,
    starts_with_use_client,
};
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn git_init(dir: &Path) {
    let output = Command::new("git")
        .args(["init", "-q", "--initial-branch=main"])
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_add_all(dir: &Path) {
    let output = Command::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git add failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write(dir: &Path, path: &str, content: &str) {
    let full = dir.join(path);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(full, content).unwrap();
}

// ── has_disable_comment ──────────────────────────────────────────────────

#[test]
fn detected_on_preceding_line() {
    let source = "// guardrails-disable-next-line my-rule\nsome code here";
    assert!(has_disable_comment(source, 2, "my-rule"));
}

#[test]
fn not_triggered_on_line_1() {
    let source = "some code here";
    assert!(!has_disable_comment(source, 1, "my-rule"));
}

#[test]
fn wrong_rule_not_matched() {
    let source = "// guardrails-disable-next-line other-rule\nsome code here";
    assert!(!has_disable_comment(source, 2, "my-rule"));
}

#[test]
fn matches_with_colon_reason() {
    let source = "// guardrails-disable-next-line my-rule: some reason here\nsome code here";
    assert!(has_disable_comment(source, 2, "my-rule"));
}

#[test]
fn matches_with_space_reason() {
    let source = "// guardrails-disable-next-line my-rule some reason here\nsome code here";
    assert!(has_disable_comment(source, 2, "my-rule"));
}

#[test]
fn not_matched_when_in_string_literal() {
    let source = "const s = \"guardrails-disable-next-line my-rule\"\nsome code here";
    assert!(!has_disable_comment(source, 2, "my-rule"));
}

#[test]
fn not_matched_for_prefix_rule_id() {
    // "my-rule-extra" should NOT match "my-rule" since it's not "my-rule:" or "my-rule "
    // But "my-rule-extra" starts with "my-rule" so we verify the check is correct
    let source = "// guardrails-disable-next-line my-rule-extra\nsome code here";
    assert!(!has_disable_comment(source, 2, "my-rule"));
}

#[test]
fn empty_source_returns_false() {
    assert!(!has_disable_comment("", 2, "my-rule"));
}

// ── has_disable_file_comment ──────────────────────────────────────────────

#[test]
fn file_disable_detected_in_leading_comment() {
    let source = "// guardrails-disable-file my-rule: intentional\nexport const x = 1";
    assert!(has_disable_file_comment(source, "my-rule"));
}

#[test]
fn file_disable_skips_leading_blank_lines() {
    let source = "\n\n// guardrails-disable-file my-rule\nexport const x = 1";
    assert!(has_disable_file_comment(source, "my-rule"));
}

#[test]
fn file_disable_skips_leading_line_comments() {
    let source =
        "// Copyright 2026\n// eslint-disable no-console\n// guardrails-disable-file my-rule\nexport const x = 1";
    assert!(has_disable_file_comment(source, "my-rule"));
}

#[test]
fn file_disable_skips_leading_block_comments() {
    let source =
        "/*\n * Copyright 2026\n */\n// guardrails-disable-file my-rule\nexport const x = 1";
    assert!(has_disable_file_comment(source, "my-rule"));
}

#[test]
fn file_disable_skips_long_leading_block_comments() {
    let header = (0..25)
        .map(|i| format!(" * line {i}\n"))
        .collect::<String>();
    let source = format!("/*\n{header} */\n// guardrails-disable-file my-rule\nexport const x = 1");
    assert!(has_disable_file_comment(&source, "my-rule"));
}

#[test]
fn file_disable_handles_same_line_block_comment_then_directive() {
    let source = "/* Copyright 2026 */ // guardrails-disable-file my-rule\nexport const x = 1";
    assert!(has_disable_file_comment(source, "my-rule"));
}

#[test]
fn file_disable_after_block_comment_trailing_code_not_matched() {
    let source = "/* Copyright 2026 */ export const x = 1\n// guardrails-disable-file my-rule";
    assert!(!has_disable_file_comment(source, "my-rule"));
}

#[test]
fn file_disable_after_multiline_block_comment_trailing_code_not_matched() {
    let source =
        "/*\n * Copyright 2026\n */ export const x = 1\n// guardrails-disable-file my-rule";
    assert!(!has_disable_file_comment(source, "my-rule"));
}

#[test]
fn file_disable_handles_bom() {
    let source = "\u{FEFF}// guardrails-disable-file my-rule\nexport const x = 1";
    assert!(has_disable_file_comment(source, "my-rule"));
}

#[test]
fn file_disable_wrong_rule_not_matched() {
    let source = "// guardrails-disable-file other-rule\nexport const x = 1";
    assert!(!has_disable_file_comment(source, "my-rule"));
}

#[test]
fn file_disable_skips_non_matching_file_directives() {
    let source = "// guardrails-disable-file other-rule\n// guardrails-disable-file my-rule\nexport const x = 1";
    assert!(has_disable_file_comment(source, "my-rule"));
}

#[test]
fn file_disable_after_code_not_matched() {
    let source = "export const x = 1\n// guardrails-disable-file my-rule";
    assert!(!has_disable_file_comment(source, "my-rule"));
}

// ── starts_with_use_client ────────────────────────────────────────────────

#[test]
fn detects_single_quote_use_client() {
    assert!(starts_with_use_client(
        "'use client'\nexport function Foo() {}"
    ));
}

#[test]
fn detects_double_quote_use_client() {
    assert!(starts_with_use_client(
        "\"use client\"\nexport function Foo() {}"
    ));
}

#[test]
fn no_match_for_server_component() {
    assert!(!starts_with_use_client("export default function Page() {}"));
}

#[test]
fn only_checks_first_200_bytes() {
    let long_prefix = "a".repeat(210);
    let source = format!("{long_prefix}'use client'");
    assert!(!starts_with_use_client(&source));
}

// ── is_test_file ──────────────────────────────────────────────────────────

#[test]
fn detects_test_suffix_ts() {
    assert!(is_test_file("web/app/foo.test.ts"));
}

#[test]
fn detects_spec_suffix_tsx() {
    assert!(is_test_file("web/app/foo.spec.tsx"));
}

#[test]
fn detects_test_mts() {
    assert!(is_test_file("backend/foo.test.mts"));
}

#[test]
fn detects_tests_directory() {
    assert!(is_test_file("web/app/__tests__/foo.ts"));
}

#[test]
fn non_test_file_not_flagged() {
    assert!(!is_test_file("web/app/page.tsx"));
    assert!(!is_test_file("web/lib/api/server/users.ts"));
}

// ── git-aware discovery ─────────────────────────────────────────────────

#[test]
fn git_visible_files_include_tracked_and_untracked_non_ignored_files() {
    let dir = TempDir::new().unwrap();
    git_init(dir.path());
    write(dir.path(), ".gitignore", "dist/\n");
    write(dir.path(), "src/tracked.mts", "");
    write(dir.path(), "dist/ignored.mts", "");
    git_add_all(dir.path());
    write(dir.path(), "src/untracked.mts", "");

    let files = git_visible_files(dir.path()).unwrap();

    assert_eq!(
        files,
        vec![".gitignore", "src/tracked.mts", "src/untracked.mts"]
    );
}

#[test]
fn discover_files_falls_back_outside_git_repositories() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "src/main.mts", "");

    let files = discover_files(dir.path(), &[]);

    assert_eq!(files, vec![dir.path().join("src/main.mts")]);
}

#[test]
fn discover_files_normalizes_dot_components() {
    let dir = TempDir::new().unwrap();
    git_init(dir.path());
    write(dir.path(), "src/main.mts", "");
    git_add_all(dir.path());

    let files = discover_files(&dir.path().join("."), &[]);

    assert_eq!(files, vec![dir.path().join("src/main.mts")]);
}

#[test]
fn discover_files_prunes_git_visible_skip_dirs() {
    let dir = TempDir::new().unwrap();
    git_init(dir.path());
    write(dir.path(), "src/main.mts", "");
    write(dir.path(), "node_modules/pkg/index.mts", "");
    write(dir.path(), "dist/bundle.mts", "");
    git_add_all(dir.path());

    let files = discover_files(dir.path(), &[]);

    assert_eq!(files, vec![dir.path().join("src/main.mts")]);
}
