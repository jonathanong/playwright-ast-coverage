const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const { chmod, mkdtemp, rm, writeFile } = require("node:fs/promises");
const { join } = require("node:path");
const { tmpdir } = require("node:os");
const test = require("node:test");

const BIN = join(__dirname, "..", "bin", "playwright-ast-coverage.js");

test("wrapper forwards args and exit codes to the native binary", async () => {
  const root = await mkdtemp(join(tmpdir(), "playwright-ast-coverage-bin-"));
  const fake = join(root, "fake-binary.js");

  try {
    await writeFile(
      fake,
      "#!/usr/bin/env node\n"
        + "console.log(process.argv.slice(2).join(','));\n"
        + "process.exit(7);\n",
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
