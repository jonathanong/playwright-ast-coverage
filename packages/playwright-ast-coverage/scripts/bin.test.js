const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const { chmod, mkdtemp, rm, writeFile } = require("node:fs/promises");
const { join } = require("node:path");
const { tmpdir } = require("node:os");

const { binaryPath, run } = require("../bin/cli");

const BIN = join(__dirname, "..", "bin", "playwright-ast-coverage.js");

test("wrapper forwards args and exit codes to the native binary", async () => {
  const root = await mkdtemp(join(tmpdir(), "playwright-ast-coverage-bin-"));
  const fake = join(root, "fake-binary.js");

  try {
    await writeFile(
      fake,
      "#!/usr/bin/env node\n" +
        "console.log(process.argv.slice(2).join(','));\n" +
        "process.exit(7);\n",
    );
    await chmod(fake, 0o755);

    const result = spawnSync(process.execPath, [BIN, "check", "--json"], {
      env: {
        ...process.env,
        PLAYWRIGHT_AST_COVERAGE_BINARY: fake,
      },
      encoding: "utf8",
    });

    assert.equal(result.status, 7);
    assert.equal(result.stdout.trim(), "check,--json");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("wrapper reports a missing native binary", () => {
  const result = spawnSync(process.execPath, [BIN], {
    env: {
      ...process.env,
      PLAYWRIGHT_AST_COVERAGE_BINARY: join(tmpdir(), "missing-playwright-ast-coverage"),
    },
    encoding: "utf8",
  });

  assert.equal(result.status, 1);
  assert.match(result.stderr, /native binary is missing/);
});

test("wrapper helpers resolve binary paths and return statuses", () => {
  const env = { PLAYWRIGHT_AST_COVERAGE_BINARY: "/tmp/custom-binary" };
  assert.equal(binaryPath(env, "linux"), "/tmp/custom-binary");
  assert.match(binaryPath({}, "win32"), /playwright-ast-coverage\.exe$/);
  assert.equal(run([], { PLAYWRIGHT_AST_COVERAGE_BINARY: join(tmpdir(), "missing-pac") }), 1);
});

test("wrapper returns native binary results", async () => {
  const root = await mkdtemp(join(tmpdir(), "playwright-ast-coverage-cli-"));
  const fake = join(root, "fake-binary");
  const killed = [];

  try {
    await writeFile(fake, "");
    assert.equal(
      run(["check"], { PLAYWRIGHT_AST_COVERAGE_BINARY: fake }, "linux", process, () => ({
        status: 9,
      })),
      9,
    );
    assert.equal(
      run([], { PLAYWRIGHT_AST_COVERAGE_BINARY: fake }, "linux", process, () => ({
        error: new Error("nope"),
      })),
      1,
    );
    assert.equal(
      run(
        [],
        { PLAYWRIGHT_AST_COVERAGE_BINARY: fake },
        "linux",
        { kill: (pid, signal) => killed.push([pid, signal]), pid: 123 },
        () => ({ signal: "SIGTERM" }),
      ),
      1,
    );
    assert.deepEqual(killed, [[123, "SIGTERM"]]);
    assert.equal(
      run([], { PLAYWRIGHT_AST_COVERAGE_BINARY: fake }, "linux", process, () => ({})),
      0,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
