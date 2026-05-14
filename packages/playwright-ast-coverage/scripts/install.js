#!/usr/bin/env node
"use strict";

const { join } = require("node:path");
const PACKAGE_ROOT = join(__dirname, "..");
const core = require("no-mistakes-core");

async function main() {
  try {
    const pkg = require(join(PACKAGE_ROOT, "package.json"));
    const destination = await core.install(
      "playwright-ast-coverage",
      "jonathanong/playwright-ast-coverage",
      {
        version: pkg.version,
        vendorDir: join(PACKAGE_ROOT, "vendor"),
        envVar: "PLAYWRIGHT_AST_COVERAGE_RELEASE_BASE_URL",
        checkExisting: true,
      },
    );
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
  ...core,
  packageVersion: (dir) => {
    const targetDir = typeof dir === "string" ? dir : PACKAGE_ROOT;
    return require(join(targetDir, "package.json")).version;
  },
  assetName: (version, target) => core.assetName("playwright-ast-coverage", version, target),
  releaseBaseUrl: (version, envVar = "PLAYWRIGHT_AST_COVERAGE_RELEASE_BASE_URL") =>
    core.releaseBaseUrl("jonathanong/playwright-ast-coverage", version, envVar),
  install: (binName, repository, options) => {
    if (typeof binName === "object" && repository === undefined) {
      // Old signature: install(options)
      const mergedOptions = {
        envVar: "PLAYWRIGHT_AST_COVERAGE_RELEASE_BASE_URL",
        ...binName,
      };
      return core.install(
        "playwright-ast-coverage",
        "jonathanong/playwright-ast-coverage",
        mergedOptions,
      );
    }
    return core.install(binName, repository, options);
  },
};
