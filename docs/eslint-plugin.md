# ESLint and Oxlint Plugins

The lint plugins enforce file-local code shapes that keep the CLI analyzers
deterministic. Use them in editors and CI; use the CLIs for project-wide graph
checks.

## `eslint-plugin-playwright-ast-coverage`

Rules for Playwright test IDs and selector conventions.

```sh
npm install --save-dev eslint-plugin-playwright-ast-coverage
```

ESLint flat config:

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

Oxlint:

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

### Rules

| Rule | Purpose |
| --- | --- |
| `playwright-ast-coverage/literals` | Requires JSX test IDs and `getByTestId()` arguments to be static. |
| `playwright-ast-coverage/defaults` | Requires prop-passed test IDs to have string-literal defaults. |
| `playwright-ast-coverage/unique` | Requires literal test IDs to be unique within a file. |
| `playwright-ast-coverage/no-empty` | Disallows empty literal test IDs. |
| `playwright-ast-coverage/consistent-attribute` | Requires one canonical test ID attribute. |
| `playwright-ast-coverage/require-interactive-test-id` | Requires test IDs on interactive JSX elements. |
| `playwright-ast-coverage/prefer-get-by-test-id` | Reports exact CSS test ID selectors passed to Playwright APIs. |
| `playwright-ast-coverage/naming-convention` | Requires literal test IDs to match a regex. |

`configs.recommended` enables `literals`, `defaults`, `no-empty`, and `unique`.
`configs.strict` also enables canonical attribute, naming, interactive element,
and `getByTestId` preference rules.

All rules default to `data-testid` and `data-pw`. Override selectors per rule:

```js
{
  "playwright-ast-coverage/literals": ["error", {
    selectorAttributes: ["data-pw", "data-qa"],
    allowDefaultedProps: true,
    allowStaticTemplates: false
  }]
}
```

Use `playwright-ast-coverage check --assert-unique-test-ids` and
`--assert-unique-html-ids` for project-wide uniqueness. The lint rule is
file-local.

## `eslint-plugin-next-to-fetch`

Rules for statically analyzable `fetch()` calls.

```sh
npm install --save-dev eslint-plugin-next-to-fetch
```

ESLint flat config:

```js
const nextToFetch = require("eslint-plugin-next-to-fetch");

module.exports = [
  {
    files: ["**/*.{js,jsx,ts,tsx,mjs,mts}"],
    plugins: { "next-to-fetch": nextToFetch },
    rules: nextToFetch.configs.recommended.rules,
  },
];
```

Oxlint:

```jsonc
{
  "jsPlugins": ["eslint-plugin-next-to-fetch"],
  "rules": {
    "next-to-fetch/static-fetch-url": "error",
    "next-to-fetch/static-fetch-method": "error"
  }
}
```

### Rules

| Rule | Purpose |
| --- | --- |
| `next-to-fetch/static-fetch-url` | Requires `fetch()` URL arguments to be string literals or expression-free templates. |
| `next-to-fetch/static-fetch-method` | Requires `fetch()` `method` options to be string literals. |

These rules prevent dynamic forms that `next-to-fetch` cannot safely map to
Next.js routes and backend API paths.
