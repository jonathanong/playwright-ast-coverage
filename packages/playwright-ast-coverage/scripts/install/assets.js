"use strict";

const { basename } = require("node:path");

const REPOSITORY = "jonathanong/playwright-ast-coverage";

function assetName(version, target) {
  const ext = target.endsWith("windows-msvc") ? ".exe" : "";
  return `playwright-ast-coverage-v${version}-${target}${ext}`;
}

function releaseBaseUrl(version) {
  return (
    process.env.PLAYWRIGHT_AST_COVERAGE_RELEASE_BASE_URL ||
    `https://github.com/${REPOSITORY}/releases/download/v${version}`
  );
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

module.exports = {
  assetName,
  parseChecksum,
  releaseBaseUrl,
};
