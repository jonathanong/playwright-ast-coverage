#!/usr/bin/env node
"use strict";

const { join } = require("node:path");
const PACKAGE_ROOT = join(__dirname, "..");
const pkg = require(join(PACKAGE_ROOT, "package.json"));
const core = require("no-mistakes-core");
const INSTALL_DEFAULTS = {
  version: pkg.version,
  vendorDir: join(PACKAGE_ROOT, "vendor"),
  envVar: "PLAYWRIGHT_AST_COVERAGE_RELEASE_BASE_URL",
  checkExisting: true,
};
const DEFAULT_BIN_NAME = "playwright-ast-coverage";
const DEFAULT_REPOSITORY = "jonathanong/no-mistakes";

function isPlatform(value) {
  return (
    typeof value === "string" &&
    [
      "aix",
      "android",
      "darwin",
      "freebsd",
      "linux",
      "netbsd",
      "openbsd",
      "sunos",
      "win32",
    ].includes(value)
  );
}

function unsupportedPlatformMessage(binNameOrPlatform, platform, arch, report) {
  if (isPlatform(binNameOrPlatform) && typeof platform === "string") {
    return core.unsupportedPlatformMessage(DEFAULT_BIN_NAME, binNameOrPlatform, platform, arch);
  }
  return core.unsupportedPlatformMessage(binNameOrPlatform, platform, arch, report);
}

async function main(installFn = core.install, io = process, logger = console) {
  try {
    const destination = await installFn(DEFAULT_BIN_NAME, DEFAULT_REPOSITORY, {
      ...INSTALL_DEFAULTS,
    });
    logger.log(`Installed playwright-ast-coverage native binary to ${destination}`);
  } catch (error) {
    logger.error(error instanceof Error ? error.message : String(error));
    io.exit(1);
  }
}

if (require.main === module) main();

module.exports = {
  ...core,
  unsupportedPlatformMessage,
  packageVersion: (dir) => {
    const targetDir = typeof dir === "string" ? dir : PACKAGE_ROOT;
    return require(join(targetDir, "package.json")).version;
  },
  assetName: (version, target) => core.assetName(DEFAULT_BIN_NAME, version, target),
  releaseBaseUrl: (version, envVar = "PLAYWRIGHT_AST_COVERAGE_RELEASE_BASE_URL") =>
    core.releaseBaseUrl(DEFAULT_REPOSITORY, version, envVar),
  install: (binName, repository, options) => {
    if (binName === undefined) {
      return core.install(DEFAULT_BIN_NAME, DEFAULT_REPOSITORY, {
        ...INSTALL_DEFAULTS,
      });
    }
    if (typeof binName === "object" && repository === undefined) {
      // Old signature: install(options)
      const mergedOptions = {
        ...INSTALL_DEFAULTS,
        ...binName,
      };
      return core.install(DEFAULT_BIN_NAME, DEFAULT_REPOSITORY, mergedOptions);
    }
    return core.install(binName, repository, {
      ...INSTALL_DEFAULTS,
      ...options,
    });
  },
  main,
};
