const assert = require("node:assert/strict");
const { createHash } = require("node:crypto");
const { createServer } = require("node:http");
const { mkdir, mkdtemp, readFile, rm, stat, writeFile } = require("node:fs/promises");
const { join } = require("node:path");
const { tmpdir } = require("node:os");
const test = require("node:test");

const {
  assetName,
  install,
  parseChecksum,
  platformTarget,
} = require("./install");

function glibcReport() {
  return {
    getReport() {
      return { header: { glibcVersionRuntime: "2.39" } };
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
  assert.equal(platformTarget("linux", "x64", glibcReport()), "x86_64-unknown-linux-gnu");
  assert.equal(platformTarget("linux", "arm64", glibcReport()), "aarch64-unknown-linux-gnu");
});

test("rejects unsupported platform targets", () => {
  assert.equal(platformTarget("linux", "x64", muslReport()), null);
  assert.equal(platformTarget("freebsd", "x64"), null);
  assert.equal(platformTarget("win32", "arm64"), null);
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

test("parses sha256 files with or without filenames", () => {
  const hash = "a".repeat(64);
  assert.equal(parseChecksum(`${hash}  binary\n`, "binary"), hash);
  assert.equal(parseChecksum(`${hash}\n`, "binary"), hash);
  assert.equal(parseChecksum(`${hash} *binary\n`, "binary"), hash);
  assert.equal(parseChecksum(`${hash}  /tmp/release/binary\n`, "binary"), hash);
  assert.throws(() => parseChecksum(`${hash} other\n`, "binary"), /No SHA-256 checksum/);
});

test("installs only the requested platform binary and verifies checksum", async () => {
  const root = await mkdtemp(join(tmpdir(), "playwright-ast-coverage-test-"));
  const vendorDir = join(root, "vendor");
  const target = "x86_64-unknown-linux-gnu";
  const version = "9.8.7";
  const asset = assetName(version, target);
  const content = Buffer.from("#!/bin/sh\nexit 0\n");
  const hash = createHash("sha256").update(content).digest("hex");

  await mkdir(join(root, "assets"));
  await writeFile(join(root, "assets", asset), content);
  await writeFile(join(root, "assets", `${asset}.sha256`), `${hash}  ${asset}\n`);
  await writeFile(join(root, "assets", "playwright-ast-coverage-v9.8.7-aarch64-apple-darwin"), "nope");

  const requests = [];
  const server = createServer(async (request, response) => {
    requests.push(request.url);
    try {
      const data = await readFile(join(root, "assets", request.url.slice(1)));
      response.writeHead(200);
      response.end(data);
    } catch {
      response.writeHead(404);
      response.end("not found");
    }
  });

  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();

  try {
    const installed = await install({
      baseUrl: `http://127.0.0.1:${address.port}`,
      target,
      vendorDir,
      version,
    });
    assert.equal(installed, join(vendorDir, "playwright-ast-coverage"));
    assert.deepEqual(requests.sort(), [`/${asset}`, `/${asset}.sha256`].sort());
    assert.equal(await readFile(installed, "utf8"), content.toString("utf8"));
    assert.equal(await readFile(join(vendorDir, "target.txt"), "utf8"), `${target}\n`);
    if (process.platform !== "win32") {
      assert.equal((await stat(installed)).mode & 0o111, 0o111);
    }
  } finally {
    await new Promise((resolve) => server.close(resolve));
    await rm(root, { recursive: true, force: true });
  }
});
