const { defineConfig } = require("vitest/config");

module.exports = defineConfig({
  test: {
    globals: true,
    include: [
      "packages/playwright-ast-coverage/scripts/*.test.js",
      "packages/queue-ast-hop/scripts/*.test.js",
      "packages/eslint-plugin-playwright-ast-coverage/test/**/*.test.mjs",
      "packages/eslint-plugin-next-to-fetch/test/**/*.test.mjs",
    ],
    coverage: {
      provider: "v8",
      include: [
        "packages/playwright-ast-coverage/bin/**/*.js",
        "packages/playwright-ast-coverage/scripts/**/*.js",
        "packages/queue-ast-hop/bin/**/*.js",
        "packages/queue-ast-hop/scripts/**/*.js",
        "packages/eslint-plugin-playwright-ast-coverage/src/**/*.js",
        "packages/eslint-plugin-next-to-fetch/src/**/*.js",
      ],
      exclude: [
        "packages/playwright-ast-coverage/bin/playwright-ast-coverage.js",
        "packages/queue-ast-hop/bin/queue-ast-hop.js",
        "packages/playwright-ast-coverage/scripts/install.js",
        "packages/queue-ast-hop/scripts/install.js",
        "packages/queue-ast-hop/scripts/**/*.test.js",
        "packages/playwright-ast-coverage/scripts/**/*.test.js",
        "packages/eslint-plugin-playwright-ast-coverage/test/**",
      ],
      reporter: ["text", "lcov"],
      thresholds: {
        statements: 100,
        branches: 100,
        functions: 100,
        lines: 100,
      },
    },
  },
});
