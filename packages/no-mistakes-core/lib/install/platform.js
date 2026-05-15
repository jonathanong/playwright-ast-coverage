"use strict";

const MIN_GLIBC = [2, 35];

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

module.exports = {
  glibcVersion,
  isGlibc,
  platformTarget,
  supportedGlibc,
};
