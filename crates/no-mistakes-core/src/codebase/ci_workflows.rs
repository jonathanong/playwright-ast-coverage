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
/// - `cargo run -p <name>` (package name treated as binary name)
/// - `cargo build --bin <name>`
/// - `./target/<profile>/<name>` (direct invocation)
/// - `<name> [args]` where `<name>` matches a known binary pattern
pub fn extract_binary_names(run: &str) -> Vec<String> {
    static CARGO_BIN: OnceLock<Regex> = OnceLock::new();
    static CARGO_P: OnceLock<Regex> = OnceLock::new();
    static TARGET_BIN: OnceLock<Regex> = OnceLock::new();

    let cargo_bin_re = CARGO_BIN
        .get_or_init(|| Regex::new(r"cargo\s+(?:run|build|test)\s+.*?--bin\s+([\w-]+)").unwrap());
    let cargo_p_re = CARGO_P
        .get_or_init(|| Regex::new(r"cargo\s+(?:run|build|test)\s+.*?-p\s+([\w-]+)").unwrap());
    let target_bin_re = TARGET_BIN.get_or_init(|| Regex::new(r"\./target/\w+/([\w-]+)").unwrap());

    let mut names: Vec<String> = Vec::new();

    for cap in cargo_bin_re.captures_iter(run) {
        names.push(cap[1].to_string());
    }
    for cap in cargo_p_re.captures_iter(run) {
        names.push(cap[1].to_string());
    }
    for cap in target_bin_re.captures_iter(run) {
        names.push(cap[1].to_string());
    }

    names.sort();
    names.dedup();
    names
}

/// Parse the `[[bin]]` entries from a `Cargo.toml` string.
///
/// Returns a map of `bin_name → src/bin/<name>.rs` path (relative).
pub fn parse_cargo_bins(cargo_toml: &str) -> Result<HashMap<String, String>> {
    #[derive(Deserialize, Default)]
    struct CargoToml {
        bin: Option<Vec<BinEntry>>,
    }
    #[derive(Deserialize)]
    struct BinEntry {
        name: String,
        path: Option<String>,
    }

    let ct: CargoToml = toml::from_str(cargo_toml)?;
    let mut map = HashMap::new();
    for entry in ct.bin.unwrap_or_default() {
        let path = entry
            .path
            .unwrap_or_else(|| format!("src/bin/{}.rs", entry.name.replace('-', "_")));
        map.insert(entry.name, path);
    }
    Ok(map)
}

#[cfg(test)]
mod tests;
