#!/usr/bin/env node

const { createHash } = require("node:crypto");
const { createWriteStream } = require("node:fs");
const { chmod, copyFile, mkdir, readFile, rename, rm, writeFile } = require("node:fs/promises");
const http = require("node:http");
const https = require("node:https");
const { basename, join } = require("node:path");
const { fileURLToPath } = require("node:url");

const PACKAGE_ROOT = join(__dirname, "..");
const VENDOR_DIR = join(PACKAGE_ROOT, "vendor");
const REPOSITORY = "jonathanong/playwright-ast-coverage";

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
    if (!isGlibc(report)) {
      return null;
    }
    return arch === "x64" ? "x86_64-unknown-linux-gnu" : "aarch64-unknown-linux-gnu";
  }
  return null;
}

function isGlibc(report = process.report) {
  const header = typeof report?.getReport === "function" ? report.getReport().header : {};
  return Boolean(header?.glibcVersionRuntime || header?.glibcVersionCompiler);
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

  return new Promise((resolve, reject) => {
    const client = url.startsWith("http://") ? http : https;
    client.get(url, (response) => {
      if ([301, 302, 303, 307, 308].includes(response.statusCode || 0)) {
        response.resume();
        if (redirects >= 5 || !response.headers.location) {
          reject(new Error(`Too many redirects while downloading ${url}`));
          return;
        }
        download(new URL(response.headers.location, url).toString(), destination, redirects + 1)
          .then(resolve, reject);
        return;
      }

      if (response.statusCode !== 200) {
        response.resume();
        reject(new Error(`Download failed for ${url}: HTTP ${response.statusCode}`));
        return;
      }

      const file = createWriteStream(destination);
      response.pipe(file);
      file.on("finish", () => file.close(resolve));
      file.on("error", reject);
    }).on("error", reject);
  });
}

async function sha256(path) {
  const data = await readFile(path);
  return createHash("sha256").update(data).digest("hex");
}

async function install(options = {}) {
  const version = options.version || packageVersion();
  const target = options.target || platformTarget();
  if (!target) {
    throw new Error(
      `Unsupported platform ${process.platform}/${process.arch}. `
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
    await writeFile(join(vendorDir, "target.txt"), `${target}\n`);
    return destination;
  } catch (error) {
    await rm(temp, { force: true });
    throw error;
  }
}

async function fetchText(url) {
  const tempDir = await import("node:os").then((os) => os.tmpdir());
  const temp = join(tempDir, `playwright-ast-coverage-${process.pid}-${Date.now()}.txt`);
  try {
    await download(url, temp);
    return await readFile(temp, "utf8");
  } finally {
    await rm(temp, { force: true });
  }
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
};
