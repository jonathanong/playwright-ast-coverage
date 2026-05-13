# eslint-plugin-playwright-ast-coverage

ESLint and Oxlint rules for keeping Playwright test IDs static, defaulted, and
consistent with `playwright-ast-coverage`.

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
    "playwright-ast-coverage/unique": "error"
  }
}
```

`configs.recommended` enables `literals`, `defaults`, `no-empty`, and `unique`.
`configs.strict` also enables canonical attribute, naming, interactive element,
and `getByTestId` preference rules.

## Rules

All rules default to checking both `data-testid` and `data-pw`. Override that
with `selectorAttributes` when your project uses a different test-id attribute:

```js
{
  "playwright-ast-coverage/literals": ["error", {
    selectorAttributes: ["data-pw", "data-qa"]
  }]
}
```

### `playwright-ast-coverage/literals`

Requires JSX test-id attributes and `getByTestId()` arguments to be static.

Options:

- `selectorAttributes`: string array. Defaults to `["data-testid", "data-pw"]`.
- `allowDefaultedProps`: boolean. Defaults to `true`. Allows values passed
  through props when the current function parameter has a string-literal default.
- `allowStaticTemplates`: boolean. Defaults to `false`. Allows template literals
  with at least one static text segment, such as `` `user-${id}` ``.

### `playwright-ast-coverage/defaults`

Requires prop-passed test IDs to have a string-literal default in the current
function parameters.

Options:

- `selectorAttributes`: string array. Defaults to `["data-testid", "data-pw"]`.

### `playwright-ast-coverage/unique`

Requires literal test IDs to be unique within the same file.

This rule is intentionally file-local because ESLint and Oxlint rules do not
have a reliable project-wide view across all linted files. For project-wide
uniqueness in CI, use:

```sh
playwright-ast-coverage check --assert-unique-selectors
```

Options:

- `selectorAttributes`: string array. Defaults to `["data-testid", "data-pw"]`.

### `playwright-ast-coverage/no-empty`

Disallows empty literal test IDs.

Options:

- `selectorAttributes`: string array. Defaults to `["data-testid", "data-pw"]`.

### `playwright-ast-coverage/consistent-attribute`

Requires a canonical test-id attribute when multiple selector attributes are
recognized.

Options:

- `selectorAttributes`: string array. Defaults to `["data-testid", "data-pw"]`.
- `canonicalAttribute`: string. Defaults to `"data-pw"`.

### `playwright-ast-coverage/require-interactive-test-id`

Requires test IDs on interactive JSX elements: `button`, `input`, `select`,
`textarea`, anchors with `href`, elements with `onClick`, and elements with
interactive ARIA roles (`button`, `checkbox`, `link`, `menuitem`, `option`,
`radio`, `switch`, `tab`, `textbox`).

Options:

- `selectorAttributes`: string array. Defaults to `["data-testid", "data-pw"]`.

### `playwright-ast-coverage/prefer-get-by-test-id`

Reports exact CSS test-id selectors passed to Playwright selector APIs, so they
can be written as `getByTestId()`.

Options:

- `selectorAttributes`: string array. Defaults to `["data-testid", "data-pw"]`.

### `playwright-ast-coverage/naming-convention`

Requires literal test IDs to match a regular expression.

Options:

- `selectorAttributes`: string array. Defaults to `["data-testid", "data-pw"]`.
- `pattern`: string. Defaults to `^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$`.
