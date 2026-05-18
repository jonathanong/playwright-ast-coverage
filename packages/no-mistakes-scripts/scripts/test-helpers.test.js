const assert = require("node:assert/strict");
const { chmod, mkdtemp, rm, writeFile } = require("node:fs/promises");
const { tmpdir } = require("node:os");
const { delimiter, join } = require("node:path");
const { hasCommand } = require("./test-helpers");

async function withPathEnv(env, callback) {
  const originalPath = process.env.PATH;
  const originalPathExt = process.env.PATHEXT;

  try {
    process.env.PATH = env.PATH;
    if ("PATHEXT" in env) {
      process.env.PATHEXT = env.PATHEXT;
    } else {
      delete process.env.PATHEXT;
    }
    await callback();
  } finally {
    process.env.PATH = originalPath;
    if (originalPathExt === undefined) {
      delete process.env.PATHEXT;
    } else {
      process.env.PATHEXT = originalPathExt;
    }
  }
}

test("hasCommand ignores empty PATH segments", async () => {
  await withPathEnv({ PATH: delimiter }, async () => {
    assert.equal(hasCommand("definitely-not-installed"), false);
  });
});

test("hasCommand checks PATHEXT command candidates", async () => {
  const tempRoot = await mkdtemp(join(tmpdir(), "no-mistakes-command-"));
  try {
    const commandPath = join(tempRoot, "fixture-tool.CMD");
    await writeFile(commandPath, "");
    await chmod(commandPath, 0o755);

    await withPathEnv({ PATH: tempRoot, PATHEXT: ".CMD;.EXE" }, async () => {
      assert.equal(hasCommand("fixture-tool"), true);
    });
  } finally {
    await rm(tempRoot, { recursive: true, force: true });
  }
});
