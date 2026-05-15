import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { Linter } from "eslint";

export const require = createRequire(import.meta.url);
export const __dirname = dirname(fileURLToPath(import.meta.url));
export const plugin = require("../src");

export function fixture(name) {
  return readFileSync(resolve(__dirname, "../../../fixtures/eslint-snippets", name), "utf8");
}

export function lint(code, rules, filename = "fixture.jsx") {
  const linter = new Linter({ configType: "flat" });
  return linter.verify(
    code,
    {
      files: ["**/*.{js,jsx}"],
      languageOptions: {
        ecmaVersion: 2024,
        sourceType: "module",
        parserOptions: { ecmaFeatures: { jsx: true } },
      },
      plugins: {
        "playwright-ast-coverage": plugin,
      },
      rules,
    },
    { filename },
  );
}

export function messages(code, rule, option) {
  const config =
    option === undefined
      ? { [`playwright-ast-coverage/${rule}`]: "error" }
      : { [`playwright-ast-coverage/${rule}`]: ["error", option] };
  return lint(code, config).map((message) => message.messageId);
}
