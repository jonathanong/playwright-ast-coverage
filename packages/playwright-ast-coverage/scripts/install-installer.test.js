const assert = require("node:assert/strict");
const { createHash } = require("node:crypto");
const { createServer } = require("node:http");
const { mkdir, mkdtemp, readFile, rm, stat, writeFile } = require("node:fs/promises");
const { join } = require("node:path");
const { tmpdir } = require("node:os");
const { pathToFileURL } = require("node:url");

const { assetName, install, platformTarget, unsupportedPlatformMessage } = require("./install");

function assetBaseUrl(root) {
  return pathToFileURL(join(root, "assets")).toString();
}

function executableName(target) {
  return process.platform === "win32" || target.endsWith("windows-msvc")
    ? "playwright-ast-coverage.exe"
    : "playwright-ast-coverage";
}

const binName = "playwright-ast-coverage";
const repository = "jonathanong/playwright-ast-coverage";
const version = "9.8.7";

test("rejects unsupported install targets", async () => {
  await assert.rejects(() => install({ target: null, version }), /Unsupported platform/);
  assert.match(
    unsupportedPlatformMessage(binName, "freebsd", "x64"),
    /Unsupported platform freebsd\/x64/,
  );
  assert.match(
    unsupportedPlatformMessage(binName, "linux", "x64", { getReport: () => ({ header: {} }) }),
    /glibc 2\.35/,
  );
  assert.match(
    unsupportedPlatformMessage(binName, "linux", "arm64", { getReport: () => ({ header: {} }) }),
    /glibc 2\.35/,
  );
});

test("installs only the requested platform binary and verifies checksum", async () => {
  const root = await mkdtemp(join(tmpdir(), "playwright-ast-coverage-test-"));
  const vendorDir = join(root, "vendor");
  const target = "x86_64-unknown-linux-gnu";
  const asset = assetName(version, target);
  const content = Buffer.from("#!/bin/sh\nexit 0\n");
  const hash = createHash("sha256").update(content).digest("hex");

  await mkdir(join(root, "assets"));
  await writeFile(join(root, "assets", asset), content);
  await writeFile(join(root, "assets", `${asset}.sha256`), `${hash}  ${asset}\n`);
  await writeFile(
    join(root, "assets", "playwright-ast-coverage-v9.8.7-aarch64-apple-darwin"),
    "nope",
  );

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
    assert.equal(installed, join(vendorDir, executableName(target)));
    assert.deepEqual(requests.sort(), [`/${asset}`, `/${asset}.sha256`].sort());
    assert.equal(await readFile(installed, "utf8"), content.toString("utf8"));
    if (process.platform !== "win32") {
      assert.equal((await stat(installed)).mode & 0o111, 0o111);
    }
  } finally {
    await new Promise((resolve) => server.close(resolve));
    await rm(root, { recursive: true, force: true });
  }
});

test("installs with default target and release base environment", async () => {
  const previous = process.env.PLAYWRIGHT_AST_COVERAGE_RELEASE_BASE_URL;
  const root = await mkdtemp(join(tmpdir(), "playwright-ast-coverage-env-install-"));
  const vendorDir = join(root, "vendor");
  const target = platformTarget();
  if (!target) {
    await rm(root, { recursive: true, force: true });
    return;
  }
  const asset = assetName(version, target);
  const content = Buffer.from("#!/bin/sh\nexit 0\n");
  const hash = createHash("sha256").update(content).digest("hex");

  await mkdir(join(root, "assets"));
  await writeFile(join(root, "assets", asset), content);
  await writeFile(join(root, "assets", `${asset}.sha256`), `${hash}  ${asset}\n`);

  try {
    process.env.PLAYWRIGHT_AST_COVERAGE_RELEASE_BASE_URL = assetBaseUrl(root);
    const installed = await install({ vendorDir, version });
    assert.equal(installed, join(vendorDir, executableName(target)));
  } finally {
    if (previous === undefined) {
      delete process.env.PLAYWRIGHT_AST_COVERAGE_RELEASE_BASE_URL;
    } else {
      process.env.PLAYWRIGHT_AST_COVERAGE_RELEASE_BASE_URL = previous;
    }
    await rm(root, { recursive: true, force: true });
  }
});

test("installs Windows assets without chmod", async () => {
  const root = await mkdtemp(join(tmpdir(), "playwright-ast-coverage-windows-install-"));
  const vendorDir = join(root, "vendor");
  const target = "x86_64-pc-windows-msvc";
  const asset = assetName(version, target);
  const content = Buffer.from("windows");
  const hash = createHash("sha256").update(content).digest("hex");

  await mkdir(join(root, "assets"));
  await writeFile(join(root, "assets", asset), content);
  await writeFile(join(root, "assets", `${asset}.sha256`), `${hash}  ${asset}\n`);

  try {
    const installed = await install({
      baseUrl: assetBaseUrl(root),
      target,
      vendorDir,
      version,
    });
    assert.equal(installed, join(vendorDir, "playwright-ast-coverage.exe"));
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("rejects checksum mismatches and cleans temporary files", async () => {
  const root = await mkdtemp(join(tmpdir(), "playwright-ast-coverage-bad-checksum-"));
  const vendorDir = join(root, "vendor");
  const target = "x86_64-unknown-linux-gnu";
  const asset = assetName(version, target);
  const content = Buffer.from("#!/bin/sh\nexit 0\n");

  await mkdir(join(root, "assets"));
  await writeFile(join(root, "assets", asset), content);
  await writeFile(join(root, "assets", `${asset}.sha256`), `${"b".repeat(64)}  ${asset}\n`);

  try {
    await assert.rejects(
      () => install({ baseUrl: assetBaseUrl(root), target, vendorDir, version }),
      /Checksum mismatch/,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
