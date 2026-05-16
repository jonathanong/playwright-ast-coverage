use anyhow::Result;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

/// A single step invocation extracted from a CI workflow.
#[derive(Debug, Clone, PartialEq)]
pub struct Invocation {
    /// Optional step name (from `name:` field).
    pub step_name: Option<String>,
    /// Raw `run:` string or `uses:` string.
    pub run: String,
    /// 1-based line number (approximate; YAML parsing loses exact line info).
    pub line: u32,
    /// Binary names extracted from the run string.
    pub binaries: Vec<String>,
}

/// Extract all `run:` step invocations from a GitHub Actions workflow YAML.
pub fn extract_invocations(workflow_yaml: &str) -> Result<Vec<Invocation>> {
    let value: serde_yaml::Value = serde_yaml::from_str(workflow_yaml)?;
    let mut results = Vec::new();
    let mut line_counter: u32 = 1;

    if let Some(jobs) = value.get("jobs").and_then(|v| v.as_mapping()) {
        for (_job_id, job) in jobs {
            if let Some(steps) = job.get("steps").and_then(|v| v.as_sequence()) {
                for step in steps {
                    let step_name = step
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(str::to_string);

                    if let Some(run_str) = step.get("run").and_then(|v| v.as_str()) {
                        let binaries = extract_binary_names(run_str);
                        results.push(Invocation {
                            step_name,
                            run: run_str.to_string(),
                            line: line_counter,
                            binaries,
                        });
                        line_counter += run_str.lines().count() as u32 + 1;
                    } else if let Some(uses_str) = step.get("uses").and_then(|v| v.as_str()) {
                        results.push(Invocation {
                            step_name,
                            run: uses_str.to_string(),
                            line: line_counter,
                            binaries: vec![],
                        });
                        line_counter += 1;
                    }
                }
            }
        }
    }

    Ok(results)
}

/// Heuristically extract binary names from a shell `run:` string.
///
/// Handles:
/// - `cargo run --bin <name>`
/// - `cargo run -p <name>` or `cargo run --package <name>` (package name treated as binary name)
/// - `cargo build --bin <name>`
/// - `./target/<profile>/<name>` (direct invocation)
/// - `<name> [args]` where `<name>` matches a known binary pattern
pub fn extract_binary_names(run: &str) -> Vec<String> {
    static TARGET_BIN: OnceLock<Regex> = OnceLock::new();

    let target_bin_re = TARGET_BIN.get_or_init(|| Regex::new(r"\./target/\w+/([\w-]+)").unwrap());

    let mut names = extract_cargo_binary_names(run);
    for cap in target_bin_re.captures_iter(run) {
        names.push(cap[1].to_string());
    }

    names.sort();
    names.dedup();
    names
}

fn extract_cargo_binary_names(run: &str) -> Vec<String> {
    let tokens = shellish_words(run);
    let mut names = Vec::new();
    let mut i = 0;
    while i + 1 < tokens.len() {
        if tokens[i] != "cargo" || !is_cargo_binary_subcommand(&tokens[i + 1]) {
            i += 1;
            continue;
        }

        i += 2;
        while i < tokens.len() {
            match tokens[i].as_str() {
                "--" => break,
                "cargo" if i + 1 < tokens.len() && is_cargo_binary_subcommand(&tokens[i + 1]) => {
                    break;
                }
                "--bin" | "-p" | "--package" => {
                    if let Some(name) = tokens.get(i + 1).filter(|name| is_cargo_target_name(name))
                    {
                        names.push(name.clone());
                    }
                    i += 2;
                }
                token if token.starts_with("--bin=") => {
                    let name = token.trim_start_matches("--bin=");
                    if is_cargo_target_name(name) {
                        names.push(name.to_string());
                    }
                    i += 1;
                }
                token if token.starts_with("-p=") => {
                    let name = token.trim_start_matches("-p=");
                    if is_cargo_target_name(name) {
                        names.push(name.to_string());
                    }
                    i += 1;
                }
                token if token.starts_with("--package=") => {
                    let name = token.trim_start_matches("--package=");
                    if is_cargo_target_name(name) {
                        names.push(name.to_string());
                    }
                    i += 1;
                }
                _ => i += 1,
            }
        }
    }
    names
}

fn shellish_words(input: &str) -> Vec<String> {
    input
        .split_whitespace()
        .filter_map(|token| {
            let token = token.trim_matches(|c| matches!(c, '"' | '\'' | ';' | '\\'));
            (!token.is_empty()).then(|| token.to_string())
        })
        .collect()
}

fn is_cargo_binary_subcommand(token: &str) -> bool {
    matches!(token, "run" | "build" | "test")
}

fn is_cargo_target_name(token: &str) -> bool {
    token
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Parse workspace member entries from a root `Cargo.toml` string.
pub fn parse_cargo_workspace_members(cargo_toml: &str) -> Result<Vec<String>> {
    #[derive(Deserialize, Default)]
    struct CargoToml {
        workspace: Option<Workspace>,
    }
    #[derive(Deserialize, Default)]
    struct Workspace {
        members: Option<Vec<String>>,
    }

    let ct: CargoToml = toml::from_str(cargo_toml)?;
    Ok(ct.workspace.and_then(|ws| ws.members).unwrap_or_default())
}

/// Parse the binary entries from a `Cargo.toml` string.
///
/// Returns a map of `bin_name → src/bin/<name>.rs` path (relative).
pub fn parse_cargo_bins(cargo_toml: &str) -> Result<HashMap<String, String>> {
    #[derive(Deserialize, Default)]
    struct CargoToml {
        package: Option<Package>,
        bin: Option<Vec<BinEntry>>,
    }
    #[derive(Deserialize)]
    struct Package {
        name: String,
        autobins: Option<bool>,
    }
    #[derive(Deserialize)]
    struct BinEntry {
        name: String,
        path: Option<String>,
    }

    let ct: CargoToml = toml::from_str(cargo_toml)?;
    let mut map = HashMap::new();
    let package = ct.package;
    let package_name = package.as_ref().map(|pkg| pkg.name.as_str());
    if let Some(package) = &package {
        if package.autobins.unwrap_or(true) {
            map.insert(package.name.clone(), "src/main.rs".to_string());
        }
    }

    let explicit_bins = ct.bin.unwrap_or_default();
    if explicit_bins.is_empty() {
        return Ok(map);
    }

    for entry in explicit_bins {
        let path = entry.path.unwrap_or_else(|| {
            if package_name == Some(entry.name.as_str()) {
                "src/main.rs".to_string()
            } else {
                format!("src/bin/{}.rs", entry.name)
            }
        });
        map.insert(entry.name, path);
    }
    Ok(map)
}

#[cfg(test)]
mod tests;
