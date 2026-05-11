# playwright-ast-coverage

Static Playwright AST coverage for Next.js App Router projects.

The CLI scans Playwright tests for visited paths and selectors, derives route
patterns and configured test-id attributes from a Next.js app, and reports which
routes or selectors are not covered.

This CLI is intended to be a heuristic for AI coding agents to ensure that all pages
and components (with test IDs) have Playwright coverage. It is not intended as a replacement
for test coverage.

Currently, only Next.js is supported. PRs welcomed for other frameworks.

## Usage

`CLAUDE.md` or `AGENTS.md`:

```md
All non-Shadcn UI components should have at least one `data-pw` attribute.
`data-pw` attributes should be unique, mapping to a component or a component state.
All Next.js routes should have at least one Playwright test asserting the route.
All `data-pw` attributes should have at least one Playwright test asserting the test hook ID. 
```

Setup `.playwright-ast-coverage.yaml` below.

Add a git prepush hook:

```sh
playwright-ast-coverage
```

Add a CI check:

```sh
playwright-ast-coverage
```


## Install

```sh
cargo install --path .
```

## CLI

```sh
playwright-ast-coverage
playwright-ast-coverage --json
playwright-ast-coverage --mode edges --json
playwright-ast-coverage --root packages/web
playwright-ast-coverage --config config/playwright-ast-coverage.yaml
playwright-ast-coverage --playwright-config packages/web/playwright.config.ts
```

CLI options:

| Option | Default | Description |
| --- | --- | --- |
| `--root <ROOT>` | `.` | Repository or package root to analyze. Relative paths are resolved from the current working directory. |
| `--config <CONFIG>` | `.playwright-ast-coverage.yaml` under `--root`, when present | YAML config file. Relative paths are resolved from `--root`. Passing a missing file is an error. |
| `--playwright-config <PLAYWRIGHT_CONFIG>` | First existing `playwright.config.ts`, `.mts`, `.cts`, `.js`, `.mjs`, or `.cjs` under `--root` | Playwright config file. Relative paths are resolved from `--root`. This overrides `playwrightConfig` in the YAML config. Passing a missing file is an error. |
| `--mode <MODE>` | `coverage` | `coverage` prints coverage and exits `1` when routes or selectors are uncovered. `edges` prints detected test-to-app links and always exits `0` when analysis succeeds. |
| `--json` | `false` | Emit pretty-printed JSON instead of text output. |
| `-h`, `--help` | | Print CLI help. |
| `-V`, `--version` | | Print the package version. |

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
navigationHelpers:
  - navigateTo
selectorAttributes:
  - data-testid
  - data-pw
selectorRoots:
  - web/app
  - web/components
selectorInclude: []
selectorExclude:
  - '**/*.test.tsx'
  - '**/*.stories.tsx'
  - '**/__tests__/**'
```

YAML options:

| Option | Default | Description |
| --- | --- | --- |
| `frontendRoot` | `app` | Next.js App Router root containing `page.ts`, `page.tsx`, `page.js`, and `page.jsx` files. Relative to `--root`. |
| `playwrightConfig` | First default Playwright config found under `--root`, otherwise none | Playwright config path. Relative to `--root`. Overridden by `--playwright-config`. |
| `testInclude` | `[]` | Root-relative glob patterns for test files. When non-empty, this replaces test discovery from Playwright `testDir` and `testMatch`. |
| `testExclude` | `[]` | Root-relative glob patterns for test files to ignore. Applied to `testInclude` discovery and also in addition to Playwright `testIgnore`. |
| `ignoreRoutes` | `[]` | Route patterns that should count as covered even when no test URL matches them. Uses the same route matching rules as coverage. |
| `navigationHelpers` | `[]` | Callee names for project navigation helpers. The first URL-like string literal inside each matching call is counted as a visited URL. Names can include dots, such as `testHelpers.openPath`. |
| `selectorAttributes` | `["data-testid", "data-pw"]` | JSX attributes to collect from app source and CSS attribute selectors to detect in tests. Set to `[]` to disable selector coverage. |
| `selectorRoots` | `[frontendRoot]` | Root-relative directories to scan for app selectors. Use this for shared component directories outside the App Router tree. Missing roots are skipped. |
| `selectorInclude` | `[]` | Root-relative glob patterns for app selector source files. When empty, all source files under `selectorRoots` are included before excludes. |
| `selectorExclude` | `[]` | Root-relative glob patterns for app selector source files to ignore. Useful for unit tests, stories, and generated files. |

The tool also reads a limited set of literal values from Playwright config:

| Playwright field | Description |
| --- | --- |
| `testDir` | Directory containing tests. Project values override the root value. Defaults to `.` when no Playwright config exists. |
| `testMatch` | String glob or array of string globs for tests. Project values override the root value. JavaScript regular-expression patterns are not supported. |
| `testIgnore` | String glob or array of string globs for ignored tests. Root and project values are combined. |
| `use.baseURL` or `baseURL` | Literal base URL used to normalize absolute URLs such as `http://localhost:3000/users/42` to `/users/42`. Non-literal values are ignored. |
| `use.testIdAttribute` or `testIdAttribute` | Attribute that Playwright `getByTestId(...)` uses. Defaults to `data-testid`; project values override the root value. Non-literal values are ignored. |
| `projects` | Array of project objects. Each project inherits supported root options unless the project provides its own supported value. |

## Matching Patterns

### File Globs

`testInclude`, `testExclude`, `selectorInclude`, `selectorExclude`, Playwright
`testMatch`, and Playwright `testIgnore` use `globset` glob syntax. Paths are
slash-normalized before matching.

- YAML `testInclude`, `testExclude`, `selectorInclude`, and `selectorExclude`
  are matched against root-relative paths, such as `tests/e2e/users.spec.ts`
  or `web/components/save-button.tsx`.
- Playwright `testMatch` and `testIgnore` are matched against root-relative,
  testDir-relative, and absolute paths.
- Test and selector file walking skips `.git`, `node_modules`, `target`, `dist`,
  `build`, `coverage`, and `test-results` directories.

Examples:

```yaml
testInclude:
  - tests/**/*.spec.ts
testExclude:
  - '**/fixtures/**'
selectorInclude:
  - web/**/*.tsx
selectorExclude:
  - '**/*.test.tsx'
  - '**/*.stories.tsx'
  - '**/__tests__/**'
```

When Playwright config does not provide `testMatch`, these default test globs
are used:

```yaml
- '**/*.spec.ts'
- '**/*.spec.tsx'
- '**/*.spec.js'
- '**/*.spec.jsx'
- '**/*.spec.mts'
- '**/*.spec.cts'
- '**/*.spec.mjs'
- '**/*.spec.cjs'
- '**/*.test.ts'
- '**/*.test.tsx'
- '**/*.test.js'
- '**/*.test.jsx'
- '**/*.test.mts'
- '**/*.test.cts'
- '**/*.test.mjs'
- '**/*.test.cjs'
```

### Route Patterns

Routes are derived from files named `page.ts`, `page.tsx`, `page.js`, or
`page.jsx` under `frontendRoot`.

| App Router file | Route pattern |
| --- | --- |
| `page.tsx` | `/` |
| `settings/page.tsx` | `/settings` |
| `users/[id]/page.tsx` | `/users/:id` |
| `(admin)/settings/page.tsx` | `/settings` |
| `docs/[...rest]/page.tsx` | `/docs/*` |
| `shop/[[...rest]]/page.tsx` | `/shop/**` |
| `@modal/settings/page.tsx` | `/settings` |

Route matching rules:

- Literal segments must match exactly.
- `:name` matches one path segment.
- A final `*` matches one or more path segments.
- A final `**` matches zero or more path segments.
- Queries, fragments, and a trailing slash on non-root URLs are ignored before
  matching.
- Dynamic and wildcard segments do not match empty path segments from duplicate
  slashes.
- `ignoreRoutes` entries are compared to derived route patterns, not concrete
  visited URLs. For `users/[id]/page.tsx`, use `/users/:id` rather than
  `/users/42`.

### URL Patterns

Tests cover routes when the tool finds a URL string that normalizes to a local
path and that path matches a route pattern.

Detected URL forms:

```ts
await page.goto('/users/42');
await page.goto("http://localhost:3000/users/42");
await page.goto(`/users/${id}`);
await page.click('a[href="/settings"]');
await page.click(`a[href='/settings']`);
await expect(page).toHaveURL('/settings');
await expect(page).toHaveURL(new RegExp(`/users/${id}`));
await navigateTo(page, '/settings');
await testHelpers.openPath(page, '/settings');
```

- String literals can use single quotes, double quotes, or backticks.
- Candidate URLs must start with `/`, `http://`, or `https://`; protocol-relative
  URLs such as `//example.com/path` are treated as external and ignored.
- Absolute URLs only count when they start with a literal Playwright `baseURL`;
  the base is stripped before route matching.
- External absolute URLs without a matching `baseURL` are ignored.
- `page.goto(...)` only contributes a URL when the first argument is a string or
  template literal.
- `page.click(...)` only contributes a URL when the selector contains an
  `href="..."` or `href='...'` value.
- `.toHaveURL(...)` and configured `navigationHelpers` use the first URL-like
  string literal found in the call arguments.

### Selector Patterns

Selector coverage compares selectors declared in app JSX with selectors used by
Playwright tests. App selector source files must have `.ts`, `.tsx`, `.js`, or
`.jsx` extensions.

Supported app JSX forms for each configured `selectorAttributes` value:

```tsx
<button data-testid="save" />
<button data-testid='save' />
<button data-testid={"save"} />
<button data-testid={'save'} />
<article data-testid={`user-${id}`} />
<button data-testid={id} />
```

- Quoted string values and quoted expression values are exact selectors.
- Template literals with at least one static part are fuzzy selectors. For
  example, `user-${id}` can be covered by `user-42`.
- Dynamic expressions without static template text, such as `{id}` or
  `` `${id}` ``, are reported with `unsupportedDynamic: true` and never count as
  covered.

Detected Playwright selector forms:

```ts
await page.getByTestId('save').click();
await page.getByTestId("save").click();
await page.getByTestId(`save`).click();
await page.getByTestId(/^user-/).click();
await page.locator('[data-testid="save"]').click();
await page.locator("[data-testid='save']").click();
await page.locator('[data-testid^="user-"]').click();
await page.locator('[data-testid$="-button"]').click();
await page.locator('[data-testid*="nav"]').click();
```

- `getByTestId(...)` uses Playwright `testIdAttribute`; by default this covers
  `data-testid`. If Playwright config sets `use.testIdAttribute: 'data-pw'`,
  then `getByTestId('save')` covers `data-pw="save"`.
- CSS attribute selectors are detected for attributes listed in
  `selectorAttributes`.
- CSS attribute operators map to exact (`=`), prefix (`^=`), suffix (`$=`), and
  contains (`*=`) matching.
- `getByTestId(/.../)` regex selectors are tested against static values, and
  against a generated sample for template selectors where each dynamic hole is
  replaced with `x`.
- A selector only covers an app selector when the attribute and value matcher
  both match.

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
