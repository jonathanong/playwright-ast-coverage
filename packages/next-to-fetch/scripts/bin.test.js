const assert = require("node:assert/strict");
const { chmod, mkdir, rm, writeFile } = require("node:fs/promises");
const { join } = require("node:path");
const { spawnSync } = require("node:child_process");

const PACKAGE_ROOT = join(__dirname, "..");
const BIN = join(PACKAGE_ROOT, "bin", "next-to-fetch.js");
const VENDOR = join(PACKAGE_ROOT, "vendor");
const NATIVE = join(VENDOR, process.platform === "win32" ? "next-to-fetch.exe" : "next-to-fetch");
const { binaryPath, run } = require("../bin/next-to-fetch");
const { main } = require("./install");
const { runWithChild, testInstallerFailures } = require("no-mistakes-core/lib/test-helpers");

test("wrapper helpers resolve binary paths and handle child outcomes", async () => {
  assert.match(binaryPath("win32"), /next-to-fetch\.exe$/);
  assert.match(binaryPath("linux"), /next-to-fetch$/);

  assert.deepEqual((await runWithChild(run, ["app/page.tsx"], "exit", 7, null)).exits, [7]);
  assert.deepEqual((await runWithChild(run, ["app/page.tsx"], "exit", null, "SIGTERM")).exits, [1]);
  assert.deepEqual((await runWithChild(run, ["app/page.tsx"], "exit", null, null)).exits, [0]);
  assert.deepEqual(
    (await runWithChild(run, ["app/page.tsx"], "error", new Error("nope"))).exits,
    [1],
  );
});

test("wrapper forwards args and exit status", async () => {
  try {
    await mkdir(VENDOR, { recursive: true });
    await writeFile(
      NATIVE,
      "#!/usr/bin/env node\nconsole.log(process.argv.slice(2).join(',')); process.exit(7);\n",
    );
    await chmod(NATIVE, 0o755);

    const result = spawnSync(process.execPath, [BIN, "app/page.tsx"], {
      encoding: "utf8",
    });

    assert.equal(result.status, 7);
    assert.equal(result.stdout.trim(), "app/page.tsx");
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
  await testInstallerFailures(main, assert);
});
