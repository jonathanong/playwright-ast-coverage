#!/usr/bin/env node

const { createHash } = require("node:crypto");
const { createReadStream, createWriteStream } = require("node:fs");
const { chmod, copyFile, mkdir, readFile, rename, rm } = require("node:fs/promises");
const http = require("node:http");
const https = require("node:https");
const { basename, join } = require("node:path");
const { pipeline } = require("node:stream/promises");
const { fileURLToPath } = require("node:url");

const PACKAGE_ROOT = join(__dirname, "..");
const VENDOR_DIR = join(PACKAGE_ROOT, "vendor");
const REPOSITORY = "jonathanong/playwright-ast-coverage";
const DOWNLOAD_TIMEOUT_MS = 30_000;
const MIN_GLIBC = [2, 35];

function packageVersion() {
  return require(join(PACKAGE_ROOT, "package.json")).version;
}

function platformTarget(platform = process.platform, arch = process.arch, report = process.report) {
  if (platform === "darwin" && arch === "x64") {
    return "x86_64-apple-darwin";
  }
  if (platform === "darwin" && arch === "arm64") {
    return "aarch64-apple-darwin";
  }
  if (platform === "win32" && arch === "x64") {
    return "x86_64-pc-windows-msvc";
  }
  if (platform === "linux" && (arch === "x64" || arch === "arm64")) {
    if (!supportedGlibc(report)) {
      return null;
    }
    return arch === "x64" ? "x86_64-unknown-linux-gnu" : "aarch64-unknown-linux-gnu";
  }
  return null;
}

function isGlibc(report = process.report) {
  return glibcVersion(report) !== null;
}

function glibcVersion(report = process.report) {
  const header = typeof report?.getReport === "function" ? report.getReport().header : {};
  return header?.glibcVersionRuntime || header?.glibcVersionCompiler || null;
}

function supportedGlibc(report = process.report) {
  const version = glibcVersion(report);
  if (!version) {
    return false;
  }
  const [major, minor] = version.split(".").map((part) => Number.parseInt(part, 10));
  if (!Number.isInteger(major) || !Number.isInteger(minor)) {
    return false;
  }
  return major > MIN_GLIBC[0] || (major === MIN_GLIBC[0] && minor >= MIN_GLIBC[1]);
}

function assetName(version, target) {
  const ext = target.endsWith("windows-msvc") ? ".exe" : "";
  return `playwright-ast-coverage-v${version}-${target}${ext}`;
}

function releaseBaseUrl(version) {
  return process.env.PLAYWRIGHT_AST_COVERAGE_RELEASE_BASE_URL
    || `https://github.com/${REPOSITORY}/releases/download/v${version}`;
}

function parseChecksum(text, expectedAsset) {
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed) {
      continue;
    }
    const [hash, file] = trimmed.split(/\s+/, 2);
    if (!/^[a-fA-F0-9]{64}$/.test(hash)) {
      continue;
    }
    const normalizedFile = file?.replace(/^\*/, "");
    if (!file || normalizedFile === expectedAsset || basename(normalizedFile) === expectedAsset) {
      return hash.toLowerCase();
    }
  }
  throw new Error(`No SHA-256 checksum found for ${expectedAsset}`);
}

function download(url, destination, redirects = 0) {
  if (url.startsWith("file://")) {
    return copyFile(fileURLToPath(url), destination);
  }

  return request(url, async (response) => {
    await pipeline(response, createWriteStream(destination));
  }, redirects);
}

function request(url, handleResponse, redirects = 0) {
  return new Promise((resolve, reject) => {
    const client = url.startsWith("http://") ? http : https;
    const req = client.get(url, (response) => {
      if ([301, 302, 303, 307, 308].includes(response.statusCode || 0)) {
        response.resume();
        if (redirects >= 5 || !response.headers.location) {
          reject(new Error(`Too many redirects while downloading ${url}`));
          return;
        }
        request(new URL(response.headers.location, url).toString(), handleResponse, redirects + 1)
          .then(resolve, reject);
        return;
      }

      if (response.statusCode !== 200) {
        response.resume();
        reject(new Error(`Download failed for ${url}: HTTP ${response.statusCode}`));
        return;
      }

      Promise.resolve(handleResponse(response)).then(resolve, reject);
    });

    req.setTimeout(DOWNLOAD_TIMEOUT_MS, () => {
      req.destroy(new Error(`Download timed out after ${DOWNLOAD_TIMEOUT_MS}ms: ${url}`));
    });
    req.on("error", reject);
  });
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
  const target = options.target || platformTarget();
  if (!target) {
    throw new Error(
      `Unsupported platform ${process.platform}/${process.arch}. `
        + "Linux npm installs require glibc 2.35 or newer. "
        + "Install with `cargo install playwright-ast-coverage` instead.",
    );
  }

  const asset = assetName(version, target);
  const baseUrl = options.baseUrl || releaseBaseUrl(version);
  const vendorDir = options.vendorDir || VENDOR_DIR;
  const executable = process.platform === "win32" || target.endsWith("windows-msvc")
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

async function fetchText(url) {
  if (url.startsWith("file://")) {
    return readFile(fileURLToPath(url), "utf8");
  }
  const chunks = [];
  await request(url, async (response) => {
    for await (const chunk of response) {
      chunks.push(chunk);
    }
  });
  return Buffer.concat(chunks).toString("utf8");
}

async function main() {
  try {
    const destination = await install();
    console.log(`Installed playwright-ast-coverage native binary to ${destination}`);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}

if (require.main === module) {
  main();
}

module.exports = {
  assetName,
  install,
  isGlibc,
  parseChecksum,
  platformTarget,
  supportedGlibc,
};
