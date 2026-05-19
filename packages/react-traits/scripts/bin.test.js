const assert = require("node:assert/strict");
const { EventEmitter } = require("node:events");
const { chmod, mkdir, rm, writeFile } = require("node:fs/promises");
const { join } = require("node:path");
const { spawnSync } = require("node:child_process");

const PACKAGE_ROOT = join(__dirname, "..");
const BIN = join(PACKAGE_ROOT, "bin", "react-traits.js");
const VENDOR = join(PACKAGE_ROOT, "vendor");
const NATIVE = join(VENDOR, process.platform === "win32" ? "react-traits.exe" : "react-traits");
const { binaryPath, run } = require("../bin/react-traits");
const { main } = require("./install");

function runWithChild(event, ...eventArgs) {
  const child = new EventEmitter();
  const exits = [];
  const spawnCalls = [];
  run(["analyze"], "linux", { exit: (code) => exits.push(code) }, (bin, argv, options) => {
    spawnCalls.push([bin, argv, options]);
    queueMicrotask(() => child.emit(event, ...eventArgs));
    return child;
  });
  return new Promise((resolve) => {
    setImmediate(() => resolve({ exits, spawnCalls }));
  });
}

test("wrapper helpers resolve binary paths and handle child outcomes", async () => {
  assert.match(binaryPath("win32"), /react-traits\.exe$/);
  assert.match(binaryPath("linux"), /react-traits$/);

  assert.deepEqual((await runWithChild("exit", 7, null)).exits, [7]);
  assert.deepEqual((await runWithChild("exit", null, "SIGTERM")).exits, [1]);
  assert.deepEqual((await runWithChild("exit", null, null)).exits, [0]);
  assert.deepEqual((await runWithChild("error", new Error("nope"))).exits, [1]);
});

test("wrapper forwards args and exit status", async () => {
  try {
    await mkdir(VENDOR, { recursive: true });
    await writeFile(
      NATIVE,
      "#!/usr/bin/env node\nconsole.log(process.argv.slice(2).join(',')); process.exit(7);\n",
    );
    await chmod(NATIVE, 0o755);

    const result = spawnSync(process.execPath, [BIN, "analyze", "app.tsx"], {
      encoding: "utf8",
    });

    assert.equal(result.status, 7);
    assert.equal(result.stdout.trim(), "analyze,app.tsx");
  } finally {
    await rm(VENDOR, { recursive: true, force: true });
  }
});

test("installer succeeds when binary download is skipped", async () => {
  try {
    await mkdir(VENDOR, { recursive: true });
    await writeFile(NATIVE, "already here");
    await main();
  } finally {
    await rm(VENDOR, { recursive: true, force: true });
  }
});

test("installer reports failures", async () => {
  const exits = [];
  const errors = [];
  await main(
    async () => {
      throw new Error("install failed");
    },
    { exit: (code) => exits.push(code) },
    { log() {}, error: (message) => errors.push(message) },
  );
  assert.deepEqual(exits, [1]);
  assert.deepEqual(errors, ["install failed"]);
  await main(
    async () => {
      throw "string failed";
    },
    { exit: (code) => exits.push(code) },
    { log() {}, error: (message) => errors.push(message) },
  );
  assert.deepEqual(errors.slice(-1), ["string failed"]);
});
