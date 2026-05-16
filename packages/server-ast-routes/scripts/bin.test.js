const assert = require("node:assert/strict");
const { chmod, mkdtemp, rm, writeFile } = require("node:fs/promises");
const { tmpdir } = require("node:os");
const { join } = require("node:path");
const { spawnSync } = require("node:child_process");

const BIN = join(__dirname, "..", "bin", "server-ast-routes.js");

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
