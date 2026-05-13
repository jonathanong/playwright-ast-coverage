# playwright-ast-coverage

Static Playwright AST coverage for Next.js App Router projects.

The CLI scans Playwright tests for visited routes and test-hook selectors,
derives route patterns and app selectors from source files, and reports what is
not covered. It is intended as a heuristic for AI coding agents and CI checks;
it is not a replacement for runtime test coverage.

Currently, only Next.js App Router projects are supported.

## Agent Usage

Add guidance like this to `CLAUDE.md` or `AGENTS.md`:

```md
All non-Shadcn UI components should have at least one `data-pw` attribute.
`data-pw` attributes should be unique, mapping to a component or component state.
All Next.js routes should have at least one Playwright test asserting the route.
All `data-pw` attributes should have at least one Playwright test asserting the test hook ID.
Run `playwright-ast-coverage check --json` before finishing frontend work.
```

Common agent commands:

```sh
playwright-ast-coverage check --json
playwright-ast-coverage related 'web/app/users/[id]/page.tsx'
playwright-ast-coverage edges --json
```

Use `related` when a page or component changes and you want to run the matching
Playwright tests:

```sh
changed='web/app/users/[id]/page.tsx'
tests=$(playwright-ast-coverage related "$changed")
if [ -n "$tests" ]; then
  npx playwright test $tests
fi
```

Use `check --json` for machine-readable CI or agent decisions. A failing route
coverage check looks like this:

```json
{
  "summary": {
    "totalRoutes": 2,
    "coveredRoutes": 1,
    "uncoveredRoutes": 1,
    "totalSelectors": 0,
    "coveredSelectors": 0,
    "uncoveredSelectors": 0
  },
  "routes": [
    {
      "route": "/settings",
      "file": "web/app/settings/page.tsx",
      "covered": false,
      "tests": [],
      "urls": []
    },
    {
      "route": "/users/:id",
      "file": "web/app/users/[id]/page.tsx",
      "covered": true,
      "tests": ["tests/e2e/users.spec.ts"],
      "urls": ["/users/42"]
    }
  ],
  "selectors": []
}
```

## Install

With npm:

```sh
npm install --save-dev playwright-ast-coverage
npx playwright-ast-coverage check
```

The npm package installs a small JavaScript wrapper and downloads only the native
binary for the current computer from the matching GitHub Release. The first npm
release supports:

- macOS x64 and arm64
- Linux x64 and arm64 with glibc 2.35 or newer
- Windows x64

For unsupported platforms, or when you prefer building locally, install the Rust
crate:

```sh
cargo install playwright-ast-coverage
```

## Quick Start

Create `.playwright-ast-coverage.yaml`, `.playwright-ast-coverage.yml`,
`.playwright-ast-coverage.json`, or `.playwright-ast-coverage.jsonc` when your
app is not under the default `app` directory or when you want selector coverage
beyond `data-testid` and `data-pw`:

```yaml
frontendRoot: web/app
playwrightConfig: playwright.config.ts
navigationHelpers:
  - navigateTo
selectorAttributes:
  - data-testid
  - data-pw
selectorRoots:
  - web/app
  - web/components
selectorExclude:
  - '**/*.test.tsx'
  - '**/*.stories.tsx'
  - '**/__tests__/**'
```

Run locally or from a pre-push hook:

```sh
playwright-ast-coverage check
```

Text output is intended for humans:

```txt
Routes: 2
Covered routes: 1
Uncovered routes: 1
Selectors: 0
Covered selectors: 0
Uncovered selectors: 0

Uncovered routes:
  /settings  web/app/settings/page.tsx
```

Add the same command to CI. The command exits `1` when any non-ignored route or
selector is uncovered and exits `2` for configuration or parse errors.

## CLI

```sh
playwright-ast-coverage check
playwright-ast-coverage check --json
playwright-ast-coverage edges --json
playwright-ast-coverage related 'web/app/users/[id]/page.tsx'
playwright-ast-coverage related --project storybook 'web/app/users/[id]/page.tsx'
playwright-ast-coverage check --root packages/web
playwright-ast-coverage check --config config/playwright-ast-coverage.yaml
playwright-ast-coverage check --playwright-config packages/web/playwright.config.ts
```

The full command, configuration, matching, and output reference is in
[docs/cli-reference.md](docs/cli-reference.md).

## What Gets Checked

At a high level, the tool checks:

- Next.js route files named `page.ts`, `page.tsx`, `page.js`, or `page.jsx`
  under `frontendRoot`.
- Playwright test URLs from `page.goto(...)`, `page.click('a[href="..."]')`,
  `expect(page).toHaveURL(...)`, and configured navigation helper calls.
- JSX selector attributes such as `data-testid` and `data-pw` from app source.
- Playwright selector usage from `getByTestId(...)` and CSS attribute selectors
  passed to Playwright selector methods.
- Skipped and conditional Playwright test context, with flags to require active
  coverage or allow skipped coverage when needed.
- Literal Playwright config values for `testDir`, `testMatch`, `testIgnore`,
  `baseURL`, `testIdAttribute`, `projects`, and top-level config `name`.

For exact supported AST forms and limitations, see
[docs/ast-analysis.md](docs/ast-analysis.md).
