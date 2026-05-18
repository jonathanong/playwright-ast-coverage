const assert = require("node:assert/strict");
const { cp, mkdtemp, rm } = require("node:fs/promises");
const { tmpdir } = require("node:os");
const { join } = require("node:path");
const {
  escapeRegExp,
  fixture,
  hasCommand,
  isGitWorktree,
  REPO_ROOT,
  run,
} = require("./test-helpers");

test("rust-no-inline-tests prints help", async () => {
  const { stdout } = await run("no-mistakes-rust-no-inline-tests", ["--help"]);
  assert.match(stdout, /Usage:/);
  assert.match(stdout, /inline/);
});

test("rust-no-inline-tests reports missing ripgrep", async () => {
  await assert.rejects(
    run("no-mistakes-rust-no-inline-tests", [fixture("rust-no-inline-tests", "clean")], {
      env: { PATH: "/nonexistent" },
    }),
    { code: 1, stderr: /ripgrep/ },
  );
});

test("rust-no-inline-tests passes clean fixtures", { skip: !hasCommand("rg") }, async () => {
  const { stdout } = await run("no-mistakes-rust-no-inline-tests", [
    fixture("rust-no-inline-tests", "clean"),
    fixture("rust-no-inline-tests", "empty"),
  ]);
  assert.match(stdout, /No inline/);
});

test("rust-no-inline-tests fails inline module fixtures", { skip: !hasCommand("rg") }, async () => {
  await assert.rejects(
    run("no-mistakes-rust-no-inline-tests", [fixture("rust-no-inline-tests", "inline")]),
    { code: 1, stdout: /Inline #\[cfg\(test\)\]/ },
  );
});

test("rust-max-lines-per-file prints help", async () => {
  const { stdout } = await run("no-mistakes-rust-max-lines-per-file", ["--help"]);
  assert.match(stdout, /--src-max/);
  assert.match(stdout, /--test-max/);
});

test("rust-max-lines-per-file reports missing tokei", async () => {
  await assert.rejects(
    run("no-mistakes-rust-max-lines-per-file", [fixture("rust-max-lines-per-file", "small")], {
      env: { PATH: "/nonexistent" },
    }),
    { code: 1, stderr: /tokei/ },
  );
});

test(
  "rust-max-lines-per-file passes source and test fixtures",
  { skip: !hasCommand("tokei") || !hasCommand("jq") },
  async () => {
    const { stdout } = await run("no-mistakes-rust-max-lines-per-file", [
      "--src-max",
      "5",
      "--test-max",
      "5",
      fixture("rust-max-lines-per-file", "small/src"),
      fixture("rust-max-lines-per-file", "small/tests"),
    ]);
    assert.match(stdout, /within line limits/);
  },
);

test(
  "rust-max-lines-per-file accepts multiple source and test directories",
  { skip: !hasCommand("tokei") || !hasCommand("jq") },
  async () => {
    const { stdout } = await run("no-mistakes-rust-max-lines-per-file", [
      "--src-max",
      "5",
      "--test-max",
      "5",
      fixture("rust-max-lines-per-file", "small/src"),
      fixture("rust-max-lines-per-file", "excluded/src"),
      fixture("rust-max-lines-per-file", "small/tests"),
    ]);
    assert.match(stdout, /within line limits/);
  },
);

test(
  "rust-max-lines-per-file emits GitHub annotation on failure",
  { skip: !hasCommand("tokei") || !hasCommand("jq") },
  async () => {
    await assert.rejects(
      run("no-mistakes-rust-max-lines-per-file", [
        "--src-max",
        "2",
        fixture("rust-max-lines-per-file", "over/src"),
      ]),
      { code: 1, stdout: /^::error file=.*big\.rs::/m },
    );
  },
);

test(
  "rust-max-lines-per-file treats nested tests.rs as test code",
  { skip: !hasCommand("tokei") || !hasCommand("jq") },
  async () => {
    const { stdout } = await run("no-mistakes-rust-max-lines-per-file", [
      "--src-max",
      "2",
      "--test-max",
      "5",
      fixture("rust-max-lines-per-file", "nested/src"),
    ]);
    assert.match(stdout, /within line limits/);
  },
);

test(
  "rust-max-lines-per-file honors excludes",
  { skip: !hasCommand("tokei") || !hasCommand("jq") },
  async () => {
    const { stdout } = await run("no-mistakes-rust-max-lines-per-file", [
      "--src-max",
      "2",
      "--exclude",
      "vendored",
      fixture("rust-max-lines-per-file", "excluded/src"),
    ]);
    assert.match(stdout, /within line limits/);
  },
);

test("agents-md-max-size prints help", async () => {
  const { stdout } = await run("no-mistakes-agents-md-max-size", ["--help"]);
  assert.match(stdout, /AGENTS\.md/);
  assert.match(stdout, /--root/);
});

test("agents-md-max-size passes scoped fixtures", async () => {
  const { stdout } = await run("no-mistakes-agents-md-max-size", [
    "--root",
    fixture("agents-md-max-size", "pass"),
    "5",
    "200",
  ]);
  assert.match(stdout, /within size limits/);
});

test("agents-md-max-size fails scoped fixtures", async () => {
  const annotationPath = isGitWorktree(REPO_ROOT)
    ? "fixtures/no-mistakes-scripts/agents-md-max-size/fail/CLAUDE.md"
    : "CLAUDE.md";

  await assert.rejects(
    run("no-mistakes-agents-md-max-size", [
      "--root",
      fixture("agents-md-max-size", "fail"),
      "2",
      "200",
    ]),
    {
      code: 1,
      stdout: new RegExp(`::error file=${escapeRegExp(annotationPath)}::.*CLAUDE\\.md has 3 lines`),
    },
  );
});

test("agents-md-max-size checks non-git source trees", async () => {
  const tempRoot = await mkdtemp(join(tmpdir(), "no-mistakes-agents-md-max-size-"));
  try {
    await cp(fixture("agents-md-max-size", "pass"), join(tempRoot, "pass"), { recursive: true });
    await cp(fixture("agents-md-max-size", "fail"), join(tempRoot, "fail"), { recursive: true });

    const { stdout } = await run("no-mistakes-agents-md-max-size", [
      "--root",
      join(tempRoot, "pass"),
      "5",
      "200",
    ]);
    assert.match(stdout, /within size limits/);

    await assert.rejects(
      run("no-mistakes-agents-md-max-size", ["--root", join(tempRoot, "fail"), "2", "200"]),
      { code: 1, stdout: /CLAUDE\.md has 3 lines/ },
    );
  } finally {
    await rm(tempRoot, { recursive: true, force: true });
  }
});

test("agents-md-max-size rejects invalid limits", async () => {
  await assert.rejects(run("no-mistakes-agents-md-max-size", ["--root", REPO_ROOT, "nope"]), {
    code: 2,
    stderr: /positive integer/,
  });
});
