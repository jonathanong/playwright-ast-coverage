"use strict";

const { createHash } = require("node:crypto");
const { createReadStream } = require("node:fs");
const { chmod, mkdir, rename, rm } = require("node:fs/promises");
const { join } = require("node:path");

const { assetName, parseChecksum, releaseBaseUrl } = require("./assets");
const { download, fetchText } = require("./download");
const { platformTarget, supportedGlibc } = require("./platform");
const { existsSync } = require("node:fs");

async function sha256(path) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(path)) {
    hash.update(chunk);
  }
  return hash.digest("hex");
}

async function install(binName, repository, options = {}) {
  const version = options.version;
  if (!version) {
    throw new Error("version is required for install()");
  }
  const target = Object.hasOwn(options, "target") ? options.target : platformTarget();
  if (!target) {
    throw new Error(unsupportedPlatformMessage(binName));
  }

  const executable =
    process.platform === "win32" || target.endsWith("windows-msvc")
      ? `${binName}.exe`
      : binName;

  const vendorDir = options.vendorDir;
  if (!vendorDir) {
    throw new Error("vendorDir is required for install()");
  }
  const destination = join(vendorDir, executable);

  // Skip download if binary already exists (e.g. from a local build or previous install)
  // In development/local environments, we might already have the binary.
  if (process.env.SKIP_BINARY_DOWNLOAD || (options.checkExisting && existsSync(destination))) {
    return destination;
  }

  const asset = assetName(binName, version, target);
  const baseUrl = options.baseUrl || releaseBaseUrl(repository, version, options.envVar);
  const temp = `${destination}.tmp-${process.pid}`;

  await mkdir(vendorDir, { recursive: true });

  try {
    console.log(`Downloading ${binName} v${version} for ${target}...`);
    await download(`${baseUrl}/${asset}`, temp);
    
    let checksumText;
    try {
      checksumText = await fetchText(`${baseUrl}/${asset}.sha256`);
    } catch (e) {
      throw new Error(`Failed to fetch checksum for ${asset}: ${e.message}`);
    }

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
    throw new Error(`Failed to install ${binName}: ${error.message}`);
  }
}

function unsupportedPlatformMessage(
  binName,
  platform = process.platform,
  arch = process.arch,
  report = process.report,
) {
  if (platform === "linux" && (arch === "x64" || arch === "arm64") && !supportedGlibc(report)) {
    return `Linux npm installs require glibc 2.35 or newer. Install with \`cargo install ${binName}\` instead.`;
  }
  return `Unsupported platform ${platform}/${arch}. Install with \`cargo install ${binName}\` instead.`;
}

module.exports = {
  install,
  sha256,
  unsupportedPlatformMessage,
};
