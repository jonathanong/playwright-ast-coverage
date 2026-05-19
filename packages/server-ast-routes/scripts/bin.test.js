const assert = require("node:assert/strict");
const { chmod, mkdir, mkdtemp, rm, writeFile } = require("node:fs/promises");
const { tmpdir } = require("node:os");
const { join } = require("node:path");
const { spawnSync } = require("node:child_process");

const BIN = join(__dirname, "..", "bin", "server-ast-routes.js");
const VENDOR = join(__dirname, "..", "vendor");
const NATIVE = join(
  VENDOR,
  process.platform === "win32" ? "server-ast-routes.exe" : "server-ast-routes",
);
const { binaryPath, run } = require("../bin/server-ast-routes");
const { main } = require("./install");
const { runWithChildWithEnv, testInstallerFailures } = require("no-mistakes-core/lib/test-helpers");

test("wrapper helpers resolve binary paths and handle child outcomes", async () => {
  assert.equal(binaryPath({ SERVER_AST_ROUTES_BINARY: "/tmp/custom" }, "linux"), "/tmp/custom");
  assert.match(binaryPath({}, "win32"), /server-ast-routes\.exe$/);

  assert.deepEqual((await runWithChildWithEnv(run, ["routes"], "exit", 7, null)).exits, [7]);
  assert.deepEqual(
    (await runWithChildWithEnv(run, ["routes"], "exit", null, "SIGTERM")).exits,
    [1],
  );
  assert.deepEqual((await runWithChildWithEnv(run, ["routes"], "exit", null, null)).exits, [0]);
  assert.deepEqual(
    (await runWithChildWithEnv(run, ["routes"], "error", new Error("nope"))).exits,
    [1],
  );
});

test("wrapper forwards args and exit status", async () => {
  const root = await mkdtemp(join(tmpdir(), "server-ast-routes-bin-"));
  const fake = join(root, "fake.js");
  try {
    await writeFile(
      fake,
      "#!/usr/bin/env node\nconsole.log(process.argv.slice(2).join(',')); process.exit(7);\n",
    );
    await chmod(fake, 0o755);
    const result = spawnSync(process.execPath, [BIN, "edges", "--json"], {
      env: { ...process.env, SERVER_AST_ROUTES_BINARY: fake },
      encoding: "utf8",
    });
    assert.equal(result.status, 7);
    assert.equal(result.stdout.trim(), "edges,--json");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("wrapper exits nonzero when native process is signaled", async () => {
  const root = await mkdtemp(join(tmpdir(), "server-ast-routes-bin-signal-"));
  const fake = join(root, "fake.js");
  try {
    await writeFile(fake, "#!/usr/bin/env node\nprocess.kill(process.pid, 'SIGTERM');\n");
    await chmod(fake, 0o755);
    const result = spawnSync(process.execPath, [BIN], {
      env: { ...process.env, SERVER_AST_ROUTES_BINARY: fake },
      encoding: "utf8",
    });
    assert.equal(result.status, 1);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("wrapper reports spawn errors and installer can skip downloads", () => {
  const missing = spawnSync(process.execPath, [BIN], {
    env: {
      ...process.env,
      SERVER_AST_ROUTES_BINARY: join(tmpdir(), "missing-server-ast-routes"),
    },
    encoding: "utf8",
  });
  assert.equal(missing.status, 1);
  assert.match(missing.stderr, /ENOENT/);
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
  await testInstallerFailures(main, assert);
});
