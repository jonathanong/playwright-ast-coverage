# ESLint and Oxlint Plugin

`eslint-plugin-playwright-ast-coverage` catches test-hook issues while editing:
non-literal IDs, empty IDs, duplicate IDs within a file, inconsistent attributes,
missing interactive IDs, CSS selectors that should be `getByTestId`, and naming
convention violations.

## Install

```sh
npm install --save-dev eslint-plugin-playwright-ast-coverage
```

## ESLint

```js
const playwrightAstCoverage = require("eslint-plugin-playwright-ast-coverage");

module.exports = [
  {
    files: ["**/*.{js,jsx,ts,tsx}"],
    plugins: { "playwright-ast-coverage": playwrightAstCoverage },
    rules: playwrightAstCoverage.configs.strict.rules,
  },
];
```

## Oxlint

```jsonc
{
  "jsPlugins": ["eslint-plugin-playwright-ast-coverage"],
  "rules": {
    "playwright-ast-coverage/literals": "error",
    "playwright-ast-coverage/defaults": "error",
    "playwright-ast-coverage/unique": "error",
  },
}
```

`configs.recommended` enables `literals`, `defaults`, `no-empty`, and `unique`.
`configs.strict` also enables canonical attribute, naming, interactive element,
and `getByTestId` preference rules.

## Rule Options

All rules default to checking both `data-testid` and `data-pw`. Override that
with `selectorAttributes`:

```js
{
  "playwright-ast-coverage/literals": ["error", {
    selectorAttributes: ["data-pw", "data-qa"]
  }]
}
```

## Rules

| Rule                                                  | Purpose                                                                         |
| ----------------------------------------------------- | ------------------------------------------------------------------------------- |
| `playwright-ast-coverage/literals`                    | Requires JSX test-id attributes and `getByTestId()` arguments to be static.     |
| `playwright-ast-coverage/defaults`                    | Requires prop-passed test IDs to have string-literal defaults.                  |
| `playwright-ast-coverage/unique`                      | Requires literal test IDs to be unique within the same file.                    |
| `playwright-ast-coverage/no-empty`                    | Disallows empty literal test IDs.                                               |
| `playwright-ast-coverage/consistent-attribute`        | Requires a canonical test-id attribute when multiple attributes are recognized. |
| `playwright-ast-coverage/require-interactive-test-id` | Requires test IDs on interactive JSX elements.                                  |
| `playwright-ast-coverage/prefer-get-by-test-id`       | Reports exact CSS test-id selectors passed to Playwright selector APIs.         |
| `playwright-ast-coverage/naming-convention`           | Requires literal test IDs to match a configurable regular expression.           |

Use `playwright-ast-coverage check --assert-unique-selectors` for project-wide
selector uniqueness in CI. Lint rules are file-local.
