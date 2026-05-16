import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { describe, it } from "vitest";

import { __dirname, require } from "./helpers.mjs";

const oxlintBin = resolve(dirname(require.resolve("oxlint/package.json")), "bin", "oxlint");

describe("oxlint support", () => {
  it("loads the plugin through jsPlugins", () => {
    const root = mkdtempSync(join(tmpdir(), "ntf-oxlint-"));
    try {
      writeFileSync(join(root, "fixture.js"), "fetch(url);\n");
      writeFileSync(
        join(root, ".oxlintrc.json"),
        JSON.stringify({
          jsPlugins: [{ name: "next-to-fetch", specifier: resolve(__dirname, "../src/index.js") }],
          rules: { "next-to-fetch/static-fetch-url": "error" },
        }),
      );
      const result = spawnSync(
        process.execPath,
        [oxlintBin, "--config", ".oxlintrc.json", "fixture.js"],
        {
          cwd: root,
          encoding: "utf8",
        },
      );
      assert.notEqual(result.status, 0);
      assert.match(
        `${result.stderr || ""}${result.stdout || ""}`,
        /expression-free template literal/,
      );
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
});
