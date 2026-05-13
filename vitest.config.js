const { defineConfig } = require("vitest/config");

module.exports = defineConfig({
  test: {
    globals: true,
    include: [
      "scripts/*.test.js",
      "packages/eslint-plugin-playwright-ast-coverage/test/**/*.test.mjs",
    ],
    coverage: {
      provider: "v8",
      include: [
        "bin/**/*.js",
        "scripts/**/*.js",
        "packages/eslint-plugin-playwright-ast-coverage/src/**/*.js",
      ],
      exclude: [
        "bin/playwright-ast-coverage.js",
        "scripts/install.js",
        "scripts/**/*.test.js",
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
