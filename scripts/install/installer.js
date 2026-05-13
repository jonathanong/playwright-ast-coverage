"use strict";

const { createHash } = require("node:crypto");
const { createReadStream } = require("node:fs");
const { chmod, mkdir, rename, rm } = require("node:fs/promises");
const { join } = require("node:path");

const { assetName, parseChecksum, releaseBaseUrl } = require("./assets");
const { download, fetchText } = require("./download");
const { platformTarget } = require("./platform");

const PACKAGE_ROOT = join(__dirname, "..", "..");
const VENDOR_DIR = join(PACKAGE_ROOT, "vendor");

function packageVersion() {
  return require(join(PACKAGE_ROOT, "package.json")).version;
}

async function sha256(path) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(path)) {
    hash.update(chunk);
  }
  return hash.digest("hex");
}

async function install(options = {}) {
  const version = options.version || packageVersion();
  const target = Object.hasOwn(options, "target") ? options.target : platformTarget();
  if (!target) {
    throw new Error(
      `Unsupported platform ${process.platform}/${process.arch}. ` +
        "Linux npm installs require glibc 2.35 or newer. " +
        "Install with `cargo install playwright-ast-coverage` instead.",
    );
  }

  const asset = assetName(version, target);
  const baseUrl = options.baseUrl || releaseBaseUrl(version);
  /* v8 ignore next -- default package vendor dir is reserved for npm postinstall */
  const vendorDir = options.vendorDir || VENDOR_DIR;
  const executable =
    process.platform === "win32" || target.endsWith("windows-msvc")
      ? "playwright-ast-coverage.exe"
      : "playwright-ast-coverage";
  const destination = join(vendorDir, executable);
  const temp = `${destination}.tmp-${process.pid}`;

  await mkdir(vendorDir, { recursive: true });

  try {
    await download(`${baseUrl}/${asset}`, temp);
    const checksumText = await fetchText(`${baseUrl}/${asset}.sha256`);
    const expected = parseChecksum(checksumText, asset);
    const actual = await sha256(temp);
    if (actual !== expected) {
      throw new Error(`Checksum mismatch for ${asset}: expected ${expected}, got ${actual}`);
    }
    if (!target.endsWith("windows-msvc")) {
      await chmod(temp, 0o755);
    }
    await rename(temp, destination);
    return destination;
  } catch (error) {
    await rm(temp, { force: true });
    throw error;
  }
}

module.exports = {
  install,
  packageVersion,
  sha256,
};
