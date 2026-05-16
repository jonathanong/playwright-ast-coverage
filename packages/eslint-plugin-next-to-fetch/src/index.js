"use strict";

const rules = {
  "static-fetch-method": require("./rules/static-fetch-method"),
  "static-fetch-url": require("./rules/static-fetch-url"),
};

const plugin = {
  meta: {
    name: "eslint-plugin-next-to-fetch",
    version: require("../package.json").version,
  },
  rules,
  configs: {},
};

plugin.configs.recommended = {
  plugins: {
    "next-to-fetch": plugin,
  },
  rules: {
    "next-to-fetch/static-fetch-method": "error",
    "next-to-fetch/static-fetch-url": "error",
  },
};

plugin.configs.strict = {
  plugins: {
    "next-to-fetch": plugin,
  },
  rules: {
    ...plugin.configs.recommended.rules,
  },
};

/* v8 ignore next */
module.exports = plugin;
