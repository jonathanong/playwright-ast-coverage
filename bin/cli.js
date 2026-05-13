"use strict";

const { spawnSync } = require("node:child_process");
const { existsSync } = require("node:fs");
const { join } = require("node:path");

function binaryPath(env = process.env, platform = process.platform) {
  const executable =
    platform === "win32" ? "playwright-ast-coverage.exe" : "playwright-ast-coverage";
  return env.PLAYWRIGHT_AST_COVERAGE_BINARY || join(__dirname, "..", "vendor", executable);
}

function run(
  argv = process.argv.slice(2),
  env = process.env,
  platform = process.platform,
  io = process,
  spawn = spawnSync,
) {
  const binary = binaryPath(env, platform);
  if (!existsSync(binary)) {
    console.error(
      "playwright-ast-coverage native binary is missing. Reinstall the package, " +
        "or install the Rust crate with `cargo install playwright-ast-coverage`.",
    );
    return 1;
  }

  const result = spawn(binary, argv, { stdio: "inherit" });

  if (result.error) {
    console.error(result.error.message);
    return 1;
  }

  if (result.signal) {
    io.kill(io.pid, result.signal);
    return 1;
  }

  return result.status ?? 0;
}

module.exports = {
  binaryPath,
  run,
};
