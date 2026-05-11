use crate::js_scan;
use anyhow::Result;
use std::path::{Path, PathBuf};

const DEFAULT_TEST_MATCH: &[&str] = &[
    "**/*.spec.ts",
    "**/*.spec.tsx",
    "**/*.spec.js",
    "**/*.spec.jsx",
    "**/*.spec.mts",
    "**/*.spec.cts",
    "**/*.spec.mjs",
    "**/*.spec.cjs",
    "**/*.test.ts",
    "**/*.test.tsx",
    "**/*.test.js",
    "**/*.test.jsx",
    "**/*.test.mts",
    "**/*.test.cts",
    "**/*.test.mjs",
    "**/*.test.cjs",
];
const DEFAULT_TEST_ID_ATTRIBUTE: &str = "data-testid";

pub struct PlaywrightConfig {
    pub projects: Vec<TestProject>,
}

pub struct TestProject {
    pub config_dir: PathBuf,
    pub test_dir: String,
    pub test_match: Vec<String>,
    pub test_ignore: Vec<String>,
    pub base_url: Option<String>,
    pub test_id_attribute: String,
}

struct ParsedOptions {
    test_dir: Option<String>,
    test_match: Option<Vec<String>>,
    test_ignore: Option<Vec<String>>,
    base_url: Option<String>,
    test_id_attribute: Option<String>,
}

impl PlaywrightConfig {
    pub fn base_urls(&self) -> Vec<String> {
        let mut urls: Vec<String> = self
            .projects
            .iter()
            .filter_map(|project| project.base_url.clone())
            .collect();
        urls.sort();
        urls.dedup();
        urls
    }

    pub fn test_id_attributes(&self) -> Vec<String> {
        let mut attributes: Vec<String> = self
            .projects
            .iter()
            .map(|project| project.test_id_attribute.clone())
            .collect();
        attributes.sort();
        attributes.dedup();
        attributes
    }
}

impl TestProject {
    pub fn test_dir(&self, root: &Path) -> PathBuf {
        let path = Path::new(&self.test_dir);
        if path.is_absolute() {
            path.to_path_buf()
        } else if self.config_dir.is_absolute() {
            self.config_dir.join(path)
        } else {
            root.join(&self.config_dir).join(path)
        }
    }
}

pub fn load(root: &Path, config_path: Option<&Path>) -> Result<PlaywrightConfig> {
    let Some(config_path) = config_path else {
        return Ok(default_config(root));
    };

    if !config_path.exists() {
        anyhow::bail!(
            "Playwright config does not exist: {}",
            config_path.display()
        );
    }

    let source = std::fs::read_to_string(config_path)?;
    parse(&source, config_path.parent().unwrap_or(root))
}

fn default_config(root: &Path) -> PlaywrightConfig {
    PlaywrightConfig {
        projects: vec![TestProject {
            config_dir: root.to_path_buf(),
            test_dir: ".".to_string(),
            test_match: default_test_match(),
            test_ignore: Vec::new(),
            base_url: None,
            test_id_attribute: DEFAULT_TEST_ID_ATTRIBUTE.to_string(),
        }],
    }
}

#[allow(clippy::question_mark)]
fn parse(source: &str, config_dir: &Path) -> Result<PlaywrightConfig> {
    let root_source = without_projects_value(source);
    let root_options = match parse_options(&root_source) {
        Ok(options) => options,
        Err(err) => return Err(err),
    };
    let project_sources = project_object_sources(source);

    if project_sources.is_empty() {
        return Ok(PlaywrightConfig {
            projects: vec![merge_project(config_dir, &root_options, None)],
        });
    }

    let mut projects = Vec::new();
    for project_source in project_sources {
        let project_options = match parse_options(&project_source) {
            Ok(options) => options,
            Err(err) => return Err(err),
        };
        projects.push(merge_project(
            config_dir,
            &root_options,
            Some(project_options),
        ));
    }

    Ok(PlaywrightConfig { projects })
}

fn merge_project(
    config_dir: &Path,
    root: &ParsedOptions,
    project: Option<ParsedOptions>,
) -> TestProject {
    let project = project.unwrap_or(ParsedOptions {
        test_dir: None,
        test_match: None,
        test_ignore: None,
        base_url: None,
        test_id_attribute: None,
    });

    TestProject {
        config_dir: config_dir.to_path_buf(),
        test_dir: project
            .test_dir
            .or_else(|| root.test_dir.clone())
            .unwrap_or_else(|| ".".to_string()),
        test_match: project
            .test_match
            .or_else(|| root.test_match.clone())
            .unwrap_or_else(default_test_match),
        test_ignore: combine(root.test_ignore.clone(), project.test_ignore),
        base_url: project.base_url.or_else(|| root.base_url.clone()),
        test_id_attribute: project
            .test_id_attribute
            .or_else(|| root.test_id_attribute.clone())
            .unwrap_or_else(|| DEFAULT_TEST_ID_ATTRIBUTE.to_string()),
    }
}

fn parse_options(source: &str) -> Result<ParsedOptions> {
    let use_source = property_value(source, "use");

    Ok(ParsedOptions {
        test_dir: property_value(source, "testDir")
            .map(parse_string)
            .transpose()?,
        test_match: property_value(source, "testMatch")
            .map(parse_string_or_array)
            .transpose()?,
        test_ignore: property_value(source, "testIgnore")
            .map(parse_string_or_array)
            .transpose()?,
        base_url: use_source
            .and_then(|value| property_value(value, "baseURL"))
            .or_else(|| property_value(source, "baseURL"))
            .map(parse_optional_string)
            .transpose()?
            .flatten(),
        test_id_attribute: use_source
            .and_then(|value| property_value(value, "testIdAttribute"))
            .or_else(|| property_value(source, "testIdAttribute"))
            .map(parse_optional_string)
            .transpose()?
            .flatten(),
    })
}

fn combine(left: Option<Vec<String>>, right: Option<Vec<String>>) -> Vec<String> {
    let mut values = left.unwrap_or_default();
    values.extend(right.unwrap_or_default());
    values
}

fn default_test_match() -> Vec<String> {
    DEFAULT_TEST_MATCH
        .iter()
        .map(|pattern| pattern.to_string())
        .collect()
}

fn property_value<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    let (value_start, value_end) = property_value_span(source, key)?;
    Some(source[value_start..value_end].trim())
}

fn without_projects_value(source: &str) -> String {
    let Some((value_start, value_end)) = property_value_span(source, "projects") else {
        return source.to_string();
    };
    let mut masked = String::with_capacity(source.len());
    masked.push_str(&source[..value_start]);
    masked.push_str(" []");
    masked.push_str(&source[value_end..]);
    masked
}

fn property_value_span(source: &str, key: &str) -> Option<(usize, usize)> {
    let key_mask = js_scan::mask_comments_and_strings(source);
    let value_mask = js_scan::mask_comments(source);
    let bytes = key_mask.as_bytes();
    let mut offset = 0;

    while let Some(relative) = key_mask[offset..].find(key) {
        let start = offset + relative;
        let before_ok = start == 0 || !is_ident(bytes[start - 1]);
        let after_key = start + key.len();
        let after_ok = after_key >= bytes.len() || !is_ident(bytes[after_key]);
        if !before_ok || !after_ok {
            offset = after_key;
            continue;
        }

        let mut i = after_key;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b':' {
            let value_start = i + 1;
            let value_end = find_value_end(&value_mask, value_start);
            return Some((value_start, value_end));
        }
        offset = after_key;
    }

    None
}

fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
}

fn find_value_end(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    let mut i = start;
    let mut curly = 0usize;
    let mut square = 0usize;
    let mut paren = 0usize;
    let mut quote: Option<u8> = None;
    let mut escaped = false;

    while i < bytes.len() {
        let byte = bytes[i];
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == q {
                quote = None;
            }
            i += 1;
            continue;
        }

        match byte {
            b'\'' | b'"' | b'`' => quote = Some(byte),
            b'{' => curly += 1,
            b'}' => {
                if curly == 0 {
                    break;
                }
                curly -= 1;
            }
            b'[' => square += 1,
            b']' => {
                if square == 0 {
                    break;
                }
                square -= 1;
            }
            b'(' => paren += 1,
            b')' => {
                if paren == 0 {
                    break;
                }
                paren -= 1;
            }
            b',' if curly == 0 && square == 0 && paren == 0 => break,
            _ => {}
        }
        i += 1;
    }

    i
}

fn parse_string(value: &str) -> Result<String> {
    let value = value.trim();
    let Some(quote) = value.as_bytes().first().copied() else {
        anyhow::bail!("expected string literal, found empty value");
    };
    if !matches!(quote, b'\'' | b'"' | b'`') {
        anyhow::bail!("expected string literal, found {value:?}");
    }
    let Some(end) = find_string_end(value, 0) else {
        anyhow::bail!("unterminated string literal");
    };
    Ok(value[1..end].to_string())
}

fn parse_optional_string(value: &str) -> Result<Option<String>> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let quote = value.as_bytes()[0];
    if !matches!(quote, b'\'' | b'"' | b'`') {
        return Ok(None);
    }
    parse_string(value).map(Some)
}

fn parse_string_or_array(value: &str) -> Result<Vec<String>> {
    let value = value.trim();
    if value.starts_with('[') {
        let strings = collect_strings(value)?;
        if strings.is_empty() && value.contains('/') {
            anyhow::bail!("regular-expression test patterns are not supported; use string globs");
        }
        return Ok(strings);
    }
    Ok(vec![parse_string(value)?])
}

fn collect_strings(source: &str) -> Result<Vec<String>> {
    let source = js_scan::mask_comments(source);
    let bytes = source.as_bytes();
    let mut strings = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if matches!(bytes[i], b'\'' | b'"' | b'`') {
            let Some(end) = find_string_end(&source, i) else {
                anyhow::bail!("unterminated string literal");
            };
            strings.push(source[i + 1..end].to_string());
            i = end + 1;
        } else {
            i += 1;
        }
    }
    Ok(strings)
}

fn find_string_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let quote = *bytes.get(start)?;
    let mut escaped = false;
    for (i, byte) in bytes.iter().enumerate().skip(start + 1) {
        if escaped {
            escaped = false;
        } else if *byte == b'\\' {
            escaped = true;
        } else if *byte == quote {
            return Some(i);
        }
    }
    None
}

fn project_object_sources(source: &str) -> Vec<String> {
    let Some(projects) = property_value(source, "projects") else {
        return Vec::new();
    };
    let projects = projects.trim();
    if !projects.starts_with('[') {
        return Vec::new();
    }

    split_top_level_objects(projects)
}

fn split_top_level_objects(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut objects = Vec::new();
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    let mut depth = 0usize;
    let mut object_start: Option<usize> = None;

    for (i, byte) in bytes.iter().copied().enumerate() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == q {
                quote = None;
            }
            continue;
        }

        match byte {
            b'\'' | b'"' | b'`' => quote = Some(byte),
            b'{' => {
                if depth == 0 {
                    object_start = Some(i);
                }
                depth += 1;
            }
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 && object_start.is_some() {
                    let start = object_start.take().expect("checked object_start");
                    objects.push(source[start..=i].to_string());
                }
            }
            _ => {}
        }
    }

    objects
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_test_dir_and_match() {
        let source = r#"
import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests/e2e',
  testMatch: '**/*.spec.ts',
});
"#;
        let parsed = parse(source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects[0].test_dir, "./tests/e2e");
        assert_eq!(parsed.projects[0].test_match, vec!["**/*.spec.ts"]);
    }

    #[test]
    fn parses_projects_with_inheritance() {
        let source = r#"
export default defineConfig({
  testDir: './tests',
  testIgnore: ['**/skip/**'],
  use: { baseURL: 'http://localhost:3000', testIdAttribute: 'data-pw' },
  projects: [
    { name: 'chromium', testMatch: '**/*.spec.ts' },
    { name: 'webkit', testDir: './e2e', testMatch: ['**/*.pw.ts'], use: { testIdAttribute: 'data-test' } },
  ],
});
"#;
        let parsed = parse(source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects.len(), 2);
        assert_eq!(parsed.projects[0].test_dir, "./tests");
        assert_eq!(parsed.projects[0].test_ignore, vec!["**/skip/**"]);
        assert_eq!(
            parsed.projects[0].base_url.as_deref(),
            Some("http://localhost:3000")
        );
        assert_eq!(parsed.projects[0].test_id_attribute, "data-pw");
        assert_eq!(parsed.projects[1].test_dir, "./e2e");
        assert_eq!(parsed.projects[1].test_match, vec!["**/*.pw.ts"]);
        assert_eq!(parsed.projects[1].test_id_attribute, "data-test");
        assert_eq!(parsed.test_id_attributes(), vec!["data-pw", "data-test"]);
    }

    #[test]
    fn parses_top_level_base_url_and_string_ignore() {
        let source = r#"
export default {
  baseURL: 'http://localhost:5173',
  testIgnore: '**/skip/**',
  testIdAttribute: 'data-test-id',
};
"#;
        let parsed = parse(source, Path::new("/repo")).unwrap();
        assert_eq!(
            parsed.projects[0].base_url.as_deref(),
            Some("http://localhost:5173")
        );
        assert_eq!(parsed.projects[0].test_ignore, vec!["**/skip/**"]);
        assert_eq!(parsed.projects[0].test_id_attribute, "data-test-id");
    }

    #[test]
    fn ignores_non_literal_optional_playwright_values() {
        let source = r#"
const BASE_URL = process.env.BASE_URL ?? 'http://localhost:3000';
const TEST_ID = process.env.TEST_ID ?? 'data-pw';
export default {
  testDir: './tests',
  use: { baseURL: BASE_URL, testIdAttribute: TEST_ID },
};
"#;
        let parsed = parse(source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects[0].test_dir, "./tests");
        assert_eq!(parsed.projects[0].base_url, None);
        assert_eq!(parsed.projects[0].test_id_attribute, "data-testid");
    }

    #[test]
    fn ignores_config_keys_inside_comments_and_strings() {
        let source = r#"
const example = "testDir: './wrong', testMatch: '**/*.wrong.ts'";
/*
projects: [
  { testDir: './wrong-project', testMatch: 'wrong-project.ts' },
]
*/
export default {
  testDir: './tests',
  testMatch: [
    '**/*.spec.ts',
    // '**/*.commented.ts',
  ],
  projects: [
    { name: 'chromium', testMatch: '**/*.pw.ts' },
  ],
};
"#;
        let parsed = parse(source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects.len(), 1);
        assert_eq!(parsed.projects[0].test_dir, "./tests");
        assert_eq!(parsed.projects[0].test_match, vec!["**/*.pw.ts"]);
    }

    #[test]
    fn ignores_empty_optional_playwright_values() {
        let parsed = parse(
            "export default { use: { baseURL: , testIdAttribute: } }",
            Path::new("/repo"),
        )
        .unwrap();
        assert_eq!(parsed.projects[0].base_url, None);
        assert_eq!(parsed.projects[0].test_id_attribute, "data-testid");
    }

    #[test]
    fn load_without_config_uses_default_project() {
        let parsed = load(Path::new("/repo"), None).unwrap();
        assert_eq!(parsed.projects[0].test_dir, ".");
        assert!(parsed.projects[0]
            .test_match
            .contains(&"**/*.spec.ts".to_string()));
    }

    #[test]
    fn load_missing_config_errors() {
        let err = load(Path::new("/repo"), Some(Path::new("/repo/missing.ts")))
            .err()
            .expect("expected missing config to fail");
        assert!(err.to_string().contains("does not exist"));
    }

    #[test]
    fn load_existing_config_reads_and_parses() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = dir.path().join("playwright.config.ts");
        std::fs::write(&config, "export default { testDir: './tests' }").unwrap();

        let parsed = load(dir.path(), Some(&config)).unwrap();
        assert_eq!(parsed.projects[0].test_dir, "./tests");
    }

    #[test]
    fn load_directory_config_path_returns_read_error() {
        let dir = tempfile::TempDir::new().unwrap();
        let err = load(dir.path(), Some(dir.path()))
            .err()
            .expect("expected directory config path to fail");
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn test_dir_resolves_absolute_relative_and_relative_config_dir() {
        let absolute = TestProject {
            config_dir: PathBuf::from("/repo"),
            test_dir: "/tmp/tests".to_string(),
            test_match: vec![],
            test_ignore: vec![],
            base_url: None,
            test_id_attribute: "data-testid".to_string(),
        };
        assert_eq!(
            absolute.test_dir(Path::new("/repo")),
            PathBuf::from("/tmp/tests")
        );

        let absolute_config_relative_test_dir = TestProject {
            config_dir: PathBuf::from("/repo"),
            test_dir: "tests".to_string(),
            test_match: vec![],
            test_ignore: vec![],
            base_url: None,
            test_id_attribute: "data-testid".to_string(),
        };
        assert_eq!(
            absolute_config_relative_test_dir.test_dir(Path::new("/repo")),
            PathBuf::from("/repo/tests")
        );

        let relative_config = TestProject {
            config_dir: PathBuf::from("config"),
            test_dir: "tests".to_string(),
            test_match: vec![],
            test_ignore: vec![],
            base_url: None,
            test_id_attribute: "data-testid".to_string(),
        };
        assert_eq!(
            relative_config.test_dir(Path::new("/repo")),
            PathBuf::from("/repo/config/tests")
        );
    }

    #[test]
    fn parse_accepts_spaced_property_and_escaped_string() {
        let source = r#"export default { testDir   : "tests\\e2e", testMatch: ["**/*.spec.ts"] }"#;
        let parsed = parse(source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects[0].test_dir, r#"tests\\e2e"#);
    }

    #[test]
    fn parser_rejects_unsupported_required_values() {
        assert!(parse("export default { testDir: }", Path::new("/repo")).is_err());
        assert!(parse("export default { testDir: 123 }", Path::new("/repo")).is_err());
        assert!(parse(
            "export default { testDir: 'unterminated }",
            Path::new("/repo")
        )
        .is_err());
        assert!(parse("export default { testIgnore: 123 }", Path::new("/repo")).is_err());
        assert!(parse(
            "export default { testMatch: [/.*\\.spec\\.ts/] }",
            Path::new("/repo")
        )
        .is_err());
        assert!(parse("export default { testMatch: 123 }", Path::new("/repo")).is_err());
        assert!(parse(
            "export default { testMatch: ['unterminated] }",
            Path::new("/repo")
        )
        .is_err());
        assert!(parse(
            "export default { projects: [{ testDir: 123 }] }",
            Path::new("/repo")
        )
        .is_err());
    }

    #[test]
    fn malformed_projects_value_falls_back_to_single_project() {
        let parsed = parse(
            "export default { projects: makeProjects() }",
            Path::new("/repo"),
        )
        .unwrap();
        assert_eq!(parsed.projects.len(), 1);
    }

    #[test]
    fn property_value_ignores_matching_key_without_colon() {
        assert_eq!(
            property_value("export default { testDir = 'tests' }", "testDir"),
            None
        );
    }

    #[test]
    fn property_value_skips_key_inside_larger_identifier() {
        assert_eq!(
            property_value("contestDir: 'wrong', testDir: 'tests'", "testDir"),
            Some("'tests'")
        );
    }

    #[test]
    fn root_options_ignore_project_values() {
        let source = r#"
export default {
  projects: [{ testDir: './project-tests', testMatch: '**/*.project.ts' }],
};
"#;
        let parsed = parse(source, Path::new("/repo")).unwrap();
        assert_eq!(parsed.projects[0].test_dir, "./project-tests");
        assert_eq!(parsed.projects[0].test_match, vec!["**/*.project.ts"]);
    }

    #[test]
    fn collect_strings_handles_non_string_prefixes() {
        assert_eq!(
            collect_strings("[foo, 'a', \"b\"]").unwrap(),
            vec!["a", "b"]
        );
    }

    #[test]
    fn delimiter_scanner_handles_early_closers_and_escaped_quotes() {
        assert_eq!(find_value_end("testMatch: ]", 10), 11);
        assert_eq!(find_value_end("testMatch: )", 10), 11);
        assert_eq!(find_value_end(r#"testDir: "a\"b", next: true"#, 8), 15);
        assert_eq!(find_string_end("", 0), None);
        assert_eq!(find_string_end(r#""a\"b""#, 0), Some(5));
    }

    #[test]
    fn split_projects_handles_quoted_braces_and_escapes() {
        let projects = split_top_level_objects(
            r#"[{ name: "a\"{", use: { baseURL: "x" }, testDir: "a" }, { name: 'b', testDir: 'b' }]"#,
        );
        assert_eq!(projects.len(), 2);
        assert!(split_top_level_objects("}").is_empty());
    }
}
