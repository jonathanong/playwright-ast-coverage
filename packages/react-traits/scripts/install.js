#!/usr/bin/env node
"use strict";

const { join } = require("node:path");
const { install } = require("no-mistakes-core");

const PACKAGE_ROOT = join(__dirname, "..");

async function main(installFn = install, io = process, logger = console) {
  try {
    const pkg = require(join(PACKAGE_ROOT, "package.json"));
    const destination = await installFn("react-traits", "jonathanong/no-mistakes", {
      version: pkg.version,
      vendorDir: join(PACKAGE_ROOT, "vendor"),
      envVar: "REACT_TRAITS_RELEASE_BASE_URL",
      checkExisting: true,
    });
    logger.log(`Installed react-traits native binary to ${destination}`);
  } catch (error) {
    logger.error(error instanceof Error ? error.message : String(error));
    io.exit(1);
  }
}

if (require.main === module) main();

module.exports = { main };
