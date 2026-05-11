# playwright-ast-coverage

Static Playwright AST coverage for Next.js App Router projects.

The CLI scans Playwright tests for visited paths and selectors, derives route
patterns and configured test-id attributes from a Next.js app, and reports which
routes or selectors are not covered.

## Install

```sh
cargo install --path .
```

## Usage

```sh
playwright-ast-coverage
playwright-ast-coverage --json
playwright-ast-coverage --mode edges --json
playwright-ast-coverage --root packages/web
```

By default the tool:

- reads `.playwright-ast-coverage.yaml` when present,
- reads `playwright.config.*` when present,
- analyzes `frontendRoot: app` unless configured,
- checks route coverage and selector coverage,
- checks `data-testid` and `data-pw` selectors unless configured,
- exits `1` when any non-ignored route or selector is uncovered,
- exits `2` for configuration or parse errors.

## Configuration

Create `.playwright-ast-coverage.yaml`:

```yaml
frontendRoot: web/app
playwrightConfig: playwright.config.ts
testInclude: []
testExclude: []
ignoreRoutes: []
selectorAttributes:
  - data-testid
  - data-pw
```

`testInclude` overrides test discovery from `playwright.config.*` when non-empty.
`testExclude` is applied in addition to Playwright `testIgnore`.
`ignoreRoutes` removes matching route patterns from uncovered-route failures.
`selectorAttributes: []` disables selector coverage.

Selector coverage supports static JSX values, configured custom attributes such
as `data-test`, and template-literal patterns such as
``data-testid={`user-${id}`}``. Playwright selectors are detected from
`getByTestId(...)`, configured CSS attribute selectors, and regex test IDs.

## Output

Default text output is intended for local use. Use `--json` for CI and tooling.

Coverage JSON:

```json
{
  "summary": {
    "totalRoutes": 1,
    "coveredRoutes": 1,
    "uncoveredRoutes": 0,
    "totalSelectors": 1,
    "coveredSelectors": 1,
    "uncoveredSelectors": 0
  },
  "routes": [
    {
      "route": "/users/:id",
      "file": "web/app/users/[id]/page.tsx",
      "covered": true,
      "tests": ["tests/e2e/users.spec.ts"],
      "urls": ["/users/42"]
    }
  ],
  "selectors": [
    {
      "attribute": "data-testid",
      "value": "user-${id}",
      "file": "web/app/users/[id]/page.tsx",
      "covered": true,
      "unsupportedDynamic": false,
      "tests": ["tests/e2e/users.spec.ts"],
      "selectors": ["getByTestId(user-42)"]
    }
  ]
}
```

Edge JSON:

```json
{
  "edges": [
    {
      "kind": "route",
      "testFile": "tests/e2e/users.spec.ts",
      "routeFile": "web/app/users/[id]/page.tsx",
      "route": "/users/:id",
      "url": "/users/42"
    },
    {
      "kind": "selector",
      "testFile": "tests/e2e/users.spec.ts",
      "appFile": "web/app/users/[id]/page.tsx",
      "attribute": "data-testid",
      "value": "user-${id}",
      "selector": "getByTestId(user-42)"
    }
  ]
}
```
