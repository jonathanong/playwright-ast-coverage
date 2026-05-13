"use strict";

const rules = {
  "consistent-attribute": require("./rules/consistent-attribute"),
  defaults: require("./rules/defaults"),
  literals: require("./rules/literals"),
  "naming-convention": require("./rules/naming-convention"),
  "no-empty": require("./rules/no-empty"),
  "prefer-get-by-test-id": require("./rules/prefer-get-by-test-id"),
  "require-interactive-test-id": require("./rules/require-interactive-test-id"),
  unique: require("./rules/unique"),
};

const plugin = {
  meta: {
    name: "eslint-plugin-playwright-ast-coverage",
    version: require("../package.json").version,
  },
  rules,
  configs: {},
};

plugin.configs.recommended = {
  plugins: {
    "playwright-ast-coverage": plugin,
  },
  rules: {
    "playwright-ast-coverage/defaults": "error",
    "playwright-ast-coverage/literals": "error",
    "playwright-ast-coverage/no-empty": "error",
    "playwright-ast-coverage/unique": "error",
  },
};

plugin.configs.strict = {
  plugins: {
    "playwright-ast-coverage": plugin,
  },
  rules: {
    ...plugin.configs.recommended.rules,
    "playwright-ast-coverage/consistent-attribute": ["error", { canonicalAttribute: "data-pw" }],
    "playwright-ast-coverage/naming-convention": "error",
    "playwright-ast-coverage/prefer-get-by-test-id": "warn",
    "playwright-ast-coverage/require-interactive-test-id": "warn",
  },
};

/* v8 ignore next */
module.exports = plugin;
