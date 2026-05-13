# playwright-ast-coverage

Static Playwright AST coverage for Next.js App Router projects.

`playwright-ast-coverage` helps teams and coding agents see which routes and
test-hook selectors are not exercised by Playwright tests. It scans source code
statically, so it is fast enough for local checks, CI guardrails, and agent
handoffs.

It is a heuristic coverage tool for Playwright/App Router workflows, not a
replacement for runtime code coverage.

## Install

```sh
npm install --save-dev playwright-ast-coverage
npx playwright-ast-coverage check
```

The npm package installs a small JavaScript wrapper and downloads the matching
native binary from GitHub Releases. Unsupported platforms can install from
Cargo:

```sh
cargo install playwright-ast-coverage
```

## Why Use It

- Find App Router pages that no Playwright test visits.
- Find `data-testid` and `data-pw` hooks that tests never assert.
- Ask for related tests when a page or component changes.
- Give AI agents a deterministic pre-finish coverage check.

```sh
playwright-ast-coverage check --json
playwright-ast-coverage related 'web/app/users/[id]/page.tsx'
playwright-ast-coverage edges --json
```

## Configure

Create `.playwright-ast-coverage.yaml`, `.playwright-ast-coverage.yml`,
`.playwright-ast-coverage.json`, or `.playwright-ast-coverage.jsonc` when your
app is not under the default `app` directory or when selectors live in shared
component folders:

```yaml
frontendRoot: web/app
playwrightConfig: playwright.config.ts
selectorRoots:
  - web/app
  - web/components
selectorExclude:
  - "**/*.test.tsx"
  - "**/*.stories.tsx"
```

## References

- [CLI reference](docs/cli-reference.md)
- [AST analysis behavior](docs/ast-analysis.md)
- [Agent guide](docs/agent-guide.md)
- [ESLint and Oxlint plugin](docs/eslint-plugin.md)
