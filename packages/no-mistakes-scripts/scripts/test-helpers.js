const { execFile, execFileSync } = require("node:child_process");
const { accessSync, constants } = require("node:fs");
const { delimiter, extname, join } = require("node:path");
const { promisify } = require("node:util");

const execFileAsync = promisify(execFile);
const PACKAGE_ROOT = join(__dirname, "..");
const REPO_ROOT = join(PACKAGE_ROOT, "..", "..");
const BIN = join(PACKAGE_ROOT, "bin");

function fixture(category, name) {
  return join(REPO_ROOT, "fixtures", "no-mistakes-scripts", category, name);
}

function hasCommand(command) {
  const names = [command];
  if (process.env.PATHEXT && extname(command) === "") {
    names.push(
      ...process.env.PATHEXT.split(";")
        .filter(Boolean)
        .map((ext) => `${command}${ext}`),
    );
  }

  return (process.env.PATH || "")
    .split(delimiter)
    .filter(Boolean)
    .some((dir) => {
      try {
        return names.some((name) => {
          try {
            accessSync(join(dir, name), constants.X_OK);
            return true;
          } catch {
            return false;
          }
        });
      } catch {
        return false;
      }
    });
}

function isGitWorktree(path) {
  try {
    execFileSync("git", ["-C", path, "rev-parse", "--is-inside-work-tree"], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

function run(script, args = [], options = {}) {
  return execFileAsync("/bin/bash", [join(BIN, script), ...args], {
    cwd: REPO_ROOT,
    env: { ...process.env, ...options.env },
  });
}

module.exports = {
  escapeRegExp: (text) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"),
  fixture,
  hasCommand,
  isGitWorktree,
  REPO_ROOT,
  run,
};
