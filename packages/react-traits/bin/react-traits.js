#!/usr/bin/env node
"use strict";

const { spawn } = require("node:child_process");
const { join } = require("node:path");

const binName = process.platform === "win32" ? "react-traits.exe" : "react-traits";
const binPath = join(__dirname, "..", "vendor", binName);

const child = spawn(binPath, process.argv.slice(2), {
  stdio: "inherit",
});

child.on("exit", (code, signal) => {
  if (code !== null) {
    process.exit(code);
  }
  if (signal !== null) {
    process.exit(1);
  }
  process.exit(0);
});

child.on("error", (error) => {
  console.error(error);
  process.exit(1);
});
