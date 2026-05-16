#!/usr/bin/env node
"use strict";

const { join } = require("node:path");
const { install } = require("no-mistakes-core");

const PACKAGE_ROOT = join(__dirname, "..");

async function main() {
  try {
    const pkg = require(join(PACKAGE_ROOT, "package.json"));
    const destination = await install("queue-ast-hop", "jonathanong/playwright-ast-coverage", {
      version: pkg.version,
      vendorDir: join(PACKAGE_ROOT, "vendor"),
      envVar: "QUEUE_AST_HOP_RELEASE_BASE_URL",
      checkExisting: true,
    });
    console.log(`Installed queue-ast-hop native binary to ${destination}`);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}

if (require.main === module) {
  main();
}
