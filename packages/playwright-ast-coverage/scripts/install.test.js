const assert = require("node:assert/strict");
const { mkdir, rm, writeFile } = require("node:fs/promises");
const { join } = require("node:path");

const {
  assetName,
  isGlibc,
  packageVersion,
  main,
  parseChecksum,
  platformTarget,
  releaseBaseUrl,
  install,
  supportedGlibc,
  unsupportedPlatformMessage,
} = require("./install");

function glibcReport(version = "2.39") {
  return {
    getReport() {
      return { header: { glibcVersionRuntime: version } };
    },
  };
}

function muslReport() {
  return {
    getReport() {
      return { header: {} };
    },
  };
}

test("maps supported platforms to Rust targets", () => {
  assert.equal(platformTarget("darwin", "x64"), "x86_64-apple-darwin");
  assert.equal(platformTarget("darwin", "arm64"), "aarch64-apple-darwin");
  assert.equal(platformTarget("win32", "x64"), "x86_64-pc-windows-msvc");
  assert.equal(platformTarget("linux", "x64", glibcReport("2.35")), "x86_64-unknown-linux-gnu");
  assert.equal(platformTarget("linux", "arm64", glibcReport("2.39")), "aarch64-unknown-linux-gnu");
});

test("installer main succeeds when binary download is skipped", async () => {
  const vendor = join(__dirname, "..", "vendor");
  const executable = join(
    vendor,
    process.platform === "win32" ? "playwright-ast-coverage.exe" : "playwright-ast-coverage",
  );
  try {
    await mkdir(vendor, { recursive: true });
    await writeFile(executable, "already here");
    await main();
  } finally {
    await rm(vendor, { recursive: true, force: true });
  }
});

test("installer main reports failures", async () => {
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

test("rejects unsupported platform targets", () => {
  assert.equal(platformTarget("linux", "x64", muslReport()), null);
  assert.equal(platformTarget("linux", "x64", glibcReport("2.31")), null);
  assert.equal(platformTarget("freebsd", "x64"), null);
  assert.equal(platformTarget("win32", "arm64"), null);
});

test("checks minimum glibc version", () => {
  assert.equal(supportedGlibc(glibcReport("2.34")), false);
  assert.equal(supportedGlibc(glibcReport("2.35")), true);
  assert.equal(supportedGlibc(glibcReport("3.0")), true);
  assert.equal(supportedGlibc(glibcReport("nope")), false);
  assert.equal(supportedGlibc(muslReport()), false);
  assert.equal(isGlibc(glibcReport("2.39")), true);
  assert.equal(isGlibc(muslReport()), false);
});

test("formats release asset names", () => {
  assert.equal(
    assetName("1.2.3", "x86_64-unknown-linux-gnu"),
    "playwright-ast-coverage-v1.2.3-x86_64-unknown-linux-gnu",
  );
  assert.equal(
    assetName("1.2.3", "x86_64-pc-windows-msvc"),
    "playwright-ast-coverage-v1.2.3-x86_64-pc-windows-msvc.exe",
  );
});

test("formats release base URLs", () => {
  const previous = process.env.PLAYWRIGHT_AST_COVERAGE_RELEASE_BASE_URL;
  try {
    delete process.env.PLAYWRIGHT_AST_COVERAGE_RELEASE_BASE_URL;
    assert.equal(
      releaseBaseUrl("1.2.3"),
      "https://github.com/jonathanong/no-mistakes/releases/download/v1.2.3",
    );
    process.env.PLAYWRIGHT_AST_COVERAGE_RELEASE_BASE_URL = "https://example.test/releases";
    assert.equal(releaseBaseUrl("1.2.3"), "https://example.test/releases");
    assert.equal(packageVersion(), require("../package.json").version);
    assert.equal(packageVersion(join(__dirname, "..")), require("../package.json").version);
  } finally {
    if (previous === undefined) {
      delete process.env.PLAYWRIGHT_AST_COVERAGE_RELEASE_BASE_URL;
    } else {
      process.env.PLAYWRIGHT_AST_COVERAGE_RELEASE_BASE_URL = previous;
    }
  }
});

test("supports legacy install overloads and unsupported platform overload", async () => {
  const vendor = join(__dirname, "..", "vendor");
  const executable = join(
    vendor,
    process.platform === "win32" ? "playwright-ast-coverage.exe" : "playwright-ast-coverage",
  );
  const custom = join(vendor, "custom-bin");

  try {
    await mkdir(vendor, { recursive: true });
    await writeFile(executable, "already here");
    await writeFile(custom, "custom");

    assert.equal(await install(), executable);
    assert.equal(await install({ checkExisting: true }), executable);
    assert.equal(
      await install("custom-bin", "owner/repo", {
        checkExisting: true,
        target: "x86_64-unknown-linux-gnu",
        vendorDir: vendor,
        version: "1.0.0",
      }),
      custom,
    );
    assert.match(
      unsupportedPlatformMessage("linux", "x64", { getReport: () => ({ header: {} }) }),
      /glibc/,
    );
  } finally {
    await rm(vendor, { recursive: true, force: true });
  }
});

test("parses sha256 files with or without filenames", () => {
  const hash = "a".repeat(64);
  assert.equal(parseChecksum(`not-a-hash binary\n${hash} binary\n`, "binary"), hash);
  assert.equal(parseChecksum(`${hash}  binary\n`, "binary"), hash);
  assert.equal(parseChecksum(`${hash}\n`, "binary"), hash);
  assert.equal(parseChecksum(`${hash} *binary\n`, "binary"), hash);
  assert.equal(parseChecksum(`${hash}  /tmp/release/binary\n`, "binary"), hash);
  assert.throws(() => parseChecksum(`${hash} other\n`, "binary"), /No SHA-256 checksum/);
});
