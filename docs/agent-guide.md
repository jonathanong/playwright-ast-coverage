# Agent Guide

Use `playwright-ast-coverage` when frontend work changes Next.js App Router
pages, Playwright tests, or components with test-hook selectors.

## Suggested Agent Instructions

Add guidance like this to `AGENTS.md`, `CLAUDE.md`, or another agent instruction
file:

```md
All non-Shadcn UI components should have at least one `data-pw` attribute.
`data-pw` attributes should be unique, mapping to a component or component state.
All Next.js routes should have at least one Playwright test asserting the route.
All `data-pw` attributes should have at least one Playwright test asserting the test hook ID.
Run `playwright-ast-coverage check --json` before finishing frontend work.
```

## Agent Workflow

Run a full check before finishing frontend work:

```sh
playwright-ast-coverage check --json
```

Use `related` when a changed page or component should drive the Playwright test
selection:

```sh
changed='web/app/users/[id]/page.tsx'
tests=$(playwright-ast-coverage related "$changed")
if [ -n "$tests" ]; then
  npx playwright test $tests
fi
```

Use `edges` when diagnosing why a route or selector is counted as covered:

```sh
playwright-ast-coverage edges --json
```

## Fixing Gaps

- For uncovered routes, add or update Playwright navigation that visits a URL
  matching the reported route pattern.
- For uncovered selectors, assert the hook with `getByTestId(...)` or a supported
  CSS attribute selector.
- For duplicate exact test IDs or HTML IDs, rename hooks so each reported value
  maps to one element or state.
- Re-run `playwright-ast-coverage check --json` after changes.

See the [CLI reference](cli-reference.md) for command options and the
[AST analysis reference](ast-analysis.md) for supported static forms.
