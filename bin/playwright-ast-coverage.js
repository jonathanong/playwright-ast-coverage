#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const { existsSync } = require("node:fs");
const { join } = require("node:path");

const executable = process.platform === "win32"
  ? "playwright-ast-coverage.exe"
  : "playwright-ast-coverage";
const binary = process.env.PLAYWRIGHT_AST_COVERAGE_BINARY
  || join(__dirname, "..", "vendor", executable);

if (!existsSync(binary)) {
  console.error(
    "playwright-ast-coverage native binary is missing. Reinstall the package, "
      + "or install the Rust crate with `cargo install playwright-ast-coverage`.",
  );
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

if (result.signal) {
  process.kill(process.pid, result.signal);
}

process.exit(result.status ?? 0);
