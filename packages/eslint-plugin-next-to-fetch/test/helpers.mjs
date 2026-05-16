import { createRequire } from "node:module";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { Linter } from "eslint";

export const require = createRequire(import.meta.url);
export const __dirname = dirname(fileURLToPath(import.meta.url));
export const plugin = require("../src");

export function lint(code, rules, filename = "fixture.js", globals = {}) {
  const linter = new Linter({ configType: "flat" });
  return linter.verify(
    code,
    {
      files: ["**/*.{js,ts}"],
      languageOptions: {
        ecmaVersion: 2024,
        sourceType: "module",
        globals,
      },
      plugins: {
        "next-to-fetch": plugin,
      },
      rules,
    },
    { filename },
  );
}

export function messages(code, rule, globals = {}) {
  return lint(code, { [`next-to-fetch/${rule}`]: "error" }, "fixture.js", globals).map(
    (m) => m.messageId,
  );
}
