#!/usr/bin/env node
"use strict";

const { assetName, parseChecksum, releaseBaseUrl } = require("./install/assets");
const { download, fetchText, isRedirectStatus, request } = require("./install/download");
const { install, packageVersion, sha256 } = require("./install/installer");
const { glibcVersion, isGlibc, platformTarget, supportedGlibc } = require("./install/platform");

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
  download,
  fetchText,
  glibcVersion,
  install,
  isGlibc,
  isRedirectStatus,
  packageVersion,
  parseChecksum,
  platformTarget,
  releaseBaseUrl,
  request,
  sha256,
  supportedGlibc,
};
