use super::*;
use std::fs;
use tempfile::TempDir;

fn make_root(files: &[&str]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for f in files {
        let path = dir.path().join(f);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "").unwrap();
    }
    dir
}

#[test]
fn playwright_webserver_string_literal() {
    let root = make_root(&["backend/server/serve.mts"]);
    let config_path = root.path().join("playwright.config.mts");
    fs::write(&config_path, "").unwrap();

    let src = r#"
export default defineConfig({
  webServer: [
    {
      command: 'NODE_ENV=test node backend/server/serve.mts',
      name: 'backend',
    },
  ],
})
"#;

    let edges = extract_spawn_edges(src, &config_path, root.path());
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].entry, root.path().join("backend/server/serve.mts"));
    assert_eq!(edges[0].spawner, config_path);
}

#[test]
fn playwright_webserver_string_literal_env_prefix() {
    let root = make_root(&["lambdas/dev-server.mts"]);
    let config_path = root.path().join("playwright.config.mts");
    fs::write(&config_path, "").unwrap();

    let src = r#"
export default defineConfig({
  webServer: [
    {
      command: 'IMAGE_LAMBDA_PORT= node lambdas/dev-server.mts',
    },
  ],
})
"#;

    let edges = extract_spawn_edges(src, &config_path, root.path());
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].entry, root.path().join("lambdas/dev-server.mts"));
}

#[test]
fn playwright_webserver_template_literal_with_interpolation() {
    // Template literal commands: quasis are concatenated, interpolations replaced with "".
    // `IMAGE_LAMBDA_PORT=${process.env.PORT} node lambdas/dev-server.mts`
    // → quasis joined: "IMAGE_LAMBDA_PORT=" + " node lambdas/dev-server.mts"
    // → tokenized: skip "IMAGE_LAMBDA_PORT=", skip "node", resolve "lambdas/dev-server.mts"
    let root = make_root(&["lambdas/dev-server.mts"]);
    let config_path = root.path().join("playwright.config.mts");
    fs::write(&config_path, "").unwrap();

    let src = r#"
export default defineConfig({
  webServer: [
    {
      command: `IMAGE_LAMBDA_PORT=${process.env.PORT} node lambdas/dev-server.mts`,
    },
  ],
})
"#;

    let edges = extract_spawn_edges(src, &config_path, root.path());
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].entry, root.path().join("lambdas/dev-server.mts"));
}

#[test]
fn playwright_webserver_multiple_entries() {
    let root = make_root(&[
        "backend/server/serve.mts",
        "cloudflare-worker/scripts/start-wrangler.mts",
    ]);
    let config_path = root.path().join("playwright.config.mts");
    fs::write(&config_path, "").unwrap();

    let src = r#"
export default defineConfig({
  webServer: [
    { command: 'NODE_ENV=test node backend/server/serve.mts' },
    { command: 'node cloudflare-worker/scripts/start-wrangler.mts' },
  ],
})
"#;

    let edges = extract_spawn_edges(src, &config_path, root.path());
    assert_eq!(edges.len(), 2);
    let entries: Vec<_> = edges.iter().map(|e| e.entry.clone()).collect();
    assert!(entries.contains(&root.path().join("backend/server/serve.mts")));
    assert!(entries.contains(
        &root
            .path()
            .join("cloudflare-worker/scripts/start-wrangler.mts")
    ));
}

#[test]
fn spawn_call_with_literal_cmd() {
    let root = make_root(&["scripts/runner.mts"]);
    let caller = root.path().join("test/setup.mts");
    fs::create_dir_all(caller.parent().unwrap()).unwrap();
    fs::write(&caller, "").unwrap();

    let src = r#"spawn('node', ['scripts/runner.mts'], { cwd: undefined })"#;
    let edges = extract_spawn_edges(src, &caller, root.path());
    // The cmd is 'node' which is a runtime prefix, not a file path.
    // The spawn call resolves the first arg, which is 'node' — not a file path.
    // So no edges (the file is in args[1], which is an array, not directly supported).
    // This tests that we don't crash; actual file-in-args case is a future enhancement.
    assert!(
        edges.is_empty(),
        "file paths in spawn args arrays are not yet supported and must produce no edge"
    );
}

#[test]
fn exec_call_resolves_shell_entry() {
    let root = make_root(&["scripts/migrate.mts"]);
    let caller = root.path().join("setup.mts");
    fs::write(&caller, "").unwrap();

    let src = r#"exec('NODE_ENV=test tsx scripts/migrate.mts')"#;
    let edges = extract_spawn_edges(src, &caller, root.path());
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].entry, root.path().join("scripts/migrate.mts"));
}

#[test]
fn skips_nonexistent_files() {
    let root = TempDir::new().unwrap();
    let caller = root.path().join("setup.mts");
    fs::write(&caller, "").unwrap();

    let src = r#"exec('node scripts/does-not-exist.mts')"#;
    let edges = extract_spawn_edges(src, &caller, root.path());
    assert!(edges.is_empty(), "nonexistent file must produce no edge");
}

#[test]
fn skips_ternary_command() {
    let root = make_root(&["server.js"]);
    let config_path = root.path().join("playwright.config.mts");
    fs::write(&config_path, "").unwrap();

    // Ternary: CI ? 'node server.js' : 'npm run dev' — not a literal, must be skipped
    let src = r#"
export default defineConfig({
  webServer: [
    { command: CI ? 'node server.js' : 'npm run dev', cwd: 'web' },
  ],
})
"#;

    let edges = extract_spawn_edges(src, &config_path, root.path());
    assert!(edges.is_empty(), "conditional command must be skipped");
}
