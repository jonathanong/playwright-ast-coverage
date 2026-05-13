# CLI Reference

`playwright-ast-coverage` scans a Next.js App Router project and reports route
and selector coverage inferred from Playwright tests.

## Commands

```sh
playwright-ast-coverage check [OPTIONS]
playwright-ast-coverage edges [OPTIONS]
playwright-ast-coverage related [OPTIONS] <FILES>...
```

- `check` prints route and selector coverage. It exits `1` when any non-ignored
  route or selector is uncovered.
- `edges` prints the discovered links from test files to route files and app
  selector files.
- `related` prints test files that directly cover the given route or selector
  source files.

## Options

| Option | Default | Description |
| --- | --- | --- |
| `--root <ROOT>` | `.` | Repository or package root to analyze. Relative paths are resolved from the current working directory. |
| `--config <CONFIG>` | `.playwright-ast-coverage.{yaml,yml,json,jsonc}` under `--root`, when present | Analyzer config file. Relative paths are resolved from `--root`. Passing a missing file is an error. |
| `--playwright-config <PLAYWRIGHT_CONFIG>` | Analyzer `playwrightConfig`, otherwise all root-level `playwright*.config.*` files under `--root` | Playwright config file. May be repeated. Relative paths are resolved from `--root`. This overrides `playwrightConfig` in analyzer config. Passing a missing file is an error. |
| `--project <PROJECT>` | | Filter by top-level Playwright config `name`, not by `projects[].name`. |
| `--json` | `false` | Emit pretty-printed JSON instead of text output. |
| `--assert-conditional-tests` | `false` | Require coverage from active tests. URLs and selectors found only in conditional tests or suites do not count. |
| `--allow-skipped-tests` | `false` | Allow URLs and selectors found in unconditionally skipped tests or suites to count. |
| `-h`, `--help` | | Print CLI help. |
| `-V`, `--version` | | Print package version. |

Options can be written after the subcommand:

```sh
playwright-ast-coverage check --json
playwright-ast-coverage related --project storybook 'web/app/users/[id]/page.tsx'
```

## Defaults

By default the tool:

- reads `.playwright-ast-coverage.{yaml,yml,json,jsonc}` when present,
- reads `playwrightConfig` from analyzer config when present, otherwise reads all
  root-level `playwright*.config.*` files when present,
- analyzes `frontendRoot: app` unless configured,
- checks route coverage and selector coverage,
- checks `data-testid` and `data-pw` selectors unless configured,
- counts coverage from active tests and conditionally skipped tests,
- ignores coverage from unconditionally skipped tests and suites,
- exits `1` when any non-ignored route or selector is uncovered,
- exits `2` for configuration or parse errors.

Skipped and conditional Playwright tests are detected statically. Unconditional
`test.skip(...)` and `test.describe.skip(...)` coverage does not count unless
`--allow-skipped-tests` is set. Conditional wrappers, `test.skip(condition, ...)`,
`.skipIf(...)`, and tests or suites inside conditional branches count by default;
set `--assert-conditional-tests` to require active coverage instead.

## Analyzer Configuration

Supported analyzer config filenames are `.playwright-ast-coverage.yaml`,
`.playwright-ast-coverage.yml`, `.playwright-ast-coverage.json`, and
`.playwright-ast-coverage.jsonc`.

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

| Option | Default | Description |
| --- | --- | --- |
| `frontendRoot` | `app` | Next.js App Router root containing `page.ts`, `page.tsx`, `page.js`, and `page.jsx` files. Relative to `--root`. |
| `playwrightConfig` | All root-level `playwright*.config.*` files found under `--root`, otherwise none | Playwright config path, or array of paths. Relative to `--root`. Overridden by repeated `--playwright-config`. |
| `testInclude` | `[]` | Root-relative glob patterns for test files. When non-empty, this replaces test discovery from Playwright `testDir` and `testMatch`. |
| `testExclude` | `[]` | Root-relative glob patterns for test files to ignore. Applied to `testInclude` discovery and also in addition to Playwright `testIgnore`. |
| `ignoreRoutes` | `[]` | Route patterns that should count as covered even when no test URL matches them. Uses the same route matching rules as coverage. |
| `navigationHelpers` | `[]` | Callee names for project navigation helpers. The first URL-like string literal inside each matching call is counted as a visited URL. Names can include dots, such as `testHelpers.openPath`. |
| `selectorAttributes` | `["data-testid", "data-pw"]` | JSX attributes to collect from app source and CSS attribute selectors to detect in tests. Set to `[]` to disable selector coverage. |
| `selectorRoots` | `[frontendRoot]` | Root-relative directories to scan for app selectors. Use this for shared component directories outside the App Router tree. Missing roots are skipped. |
| `selectorInclude` | `[]` | Root-relative glob patterns for app selector source files. When empty, all source files under `selectorRoots` are included before excludes. |
| `selectorExclude` | `[]` | Root-relative glob patterns for app selector source files to ignore. Useful for unit tests, stories, and generated files. |

## Playwright Config

The tool reads a limited set of literal values from Playwright config:

| Playwright field | Description |
| --- | --- |
| `testDir` | Directory containing tests. Project values override the root value. Defaults to `.` when no Playwright config exists. |
| `testMatch` | String glob or array of string globs for tests. Project values override the root value. JavaScript regular-expression patterns are not supported. |
| `testIgnore` | String glob or array of string globs for ignored tests. Root and project values are combined. |
| `use.baseURL` or `baseURL` | Literal base URL used to normalize absolute URLs such as `http://localhost:3000/users/42` to `/users/42`. Non-literal values are ignored. |
| `use.testIdAttribute` or `testIdAttribute` | Attribute that Playwright `getByTestId(...)` uses. Defaults to `data-testid`; project values override the root value. Non-literal values are ignored. |
| `projects` | Array of project objects. Each project inherits supported root options unless the project provides its own supported value. |

When more than one Playwright config file is analyzed, or when `--project` is
used, each analyzed config must define a unique top-level `name`. The CLI
`--project` flag filters by that config name:

```ts
export default defineConfig({
  name: 'storybook',
  testDir: './playwright/storybook',
  projects: [{ name: 'chromium' }],
})
```

In this example, use `--project storybook`. The inner `projects[].name`
(`chromium`) is still parsed for Playwright inheritance, but it is not matched
by `--project`.

## File Globs

`testInclude`, `testExclude`, `selectorInclude`, `selectorExclude`, Playwright
`testMatch`, and Playwright `testIgnore` use `globset` glob syntax. Paths are
slash-normalized before matching.

- Analyzer config `testInclude`, `testExclude`, `selectorInclude`, and `selectorExclude`
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

## Route Matching

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
- During route matching, queries, fragments, and a trailing slash on non-root
  URLs are ignored after local URL normalization.
- Dynamic and wildcard segments do not match empty path segments from duplicate
  slashes.
- `ignoreRoutes` entries are compared to derived route patterns, not concrete
  visited URLs. For `users/[id]/page.tsx`, use `/users/:id` rather than
  `/users/42`.

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
