import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { describe, it } from "vitest";

import { __dirname } from "./helpers.mjs";

describe("oxlint support", () => {
  it("loads the plugin through jsPlugins", () => {
    const root = mkdtempSync(join(tmpdir(), "pac-oxlint-"));
    try {
      writeFileSync(join(root, "fixture.jsx"), "<button data-pw={id} />;\n");
      writeFileSync(
        join(root, ".oxlintrc.json"),
        JSON.stringify({
          jsPlugins: [
            { name: "playwright-ast-coverage", specifier: resolve(__dirname, "../src/index.js") },
          ],
          rules: { "playwright-ast-coverage/literals": "error" },
        }),
      );
      const result = spawnSync(
        process.execPath,
        [
          resolve(__dirname, "../../../node_modules/oxlint/bin/oxlint"),
          "--config",
          ".oxlintrc.json",
          "fixture.jsx",
        ],
        {
          cwd: root,
          encoding: "utf8",
        },
      );
      assert.notEqual(result.status, 0);
      assert.match(`${result.stderr || ""}${result.stdout || ""}`, /literal|test ID/i);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
});
