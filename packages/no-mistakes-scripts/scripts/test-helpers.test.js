const assert = require("node:assert/strict");
const { delimiter } = require("node:path");
const { fixture, hasCommand } = require("./test-helpers");

const executableCommandPath = fixture("commands", "executable");
const directoryCommandPath = fixture("commands", "directory-candidate");
const pathExtCommandPath = fixture("commands", "pathext");

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

test("hasCommand returns false when PATH is unset", async () => {
  await withPathEnv({ PATH: undefined }, async () => {
    delete process.env.PATH;
    assert.equal(hasCommand("definitely-not-installed"), false);
  });
});

test("hasCommand ignores invalid PATH entries", async () => {
  await withPathEnv(
    { PATH: ["/definitely/does/not/exist", executableCommandPath].join(delimiter) },
    async () => {
      assert.equal(hasCommand("fixture-tool"), true);
    },
  );
});

test("hasCommand finds executable files on PATH", async () => {
  await withPathEnv({ PATH: executableCommandPath }, async () => {
    assert.equal(hasCommand("fixture-tool"), true);
  });
});

test("hasCommand ignores executable directories on PATH", async () => {
  await withPathEnv({ PATH: directoryCommandPath }, async () => {
    assert.equal(hasCommand("fixture-tool"), false);
  });
});

test("hasCommand checks PATHEXT command candidates", async () => {
  await withPathEnv({ PATH: pathExtCommandPath, PATHEXT: ".CMD;.EXE" }, async () => {
    assert.equal(hasCommand("fixture-tool"), true);
  });
});
