---
name: playwright-ast-coverage
description: Use when working on Next.js App Router frontend code that should be checked with playwright-ast-coverage, including adding route coverage, selector coverage, related Playwright test selection, or AGENTS.md guidance for this library.
---

# playwright-ast-coverage

Use this skill when frontend changes may affect Next.js App Router route coverage
or Playwright test-hook selector coverage.

## Workflow

1. Inspect `.playwright-ast-coverage.{yaml,yml,json,jsonc}` and Playwright
   config before choosing commands.
2. Run:

   ```sh
   playwright-ast-coverage check --json
   ```

3. Treat uncovered routes, uncovered selectors, and duplicate exact selectors as
   actionable failures unless project instructions explicitly allow them.
4. Fix gaps in app code or Playwright tests, then rerun the check.

## Choosing Commands

- Use `check --json` for machine-readable coverage decisions.
- Use `related <file>` to identify Playwright tests for a changed page or
  selector-bearing component.
- Use `edges --json` to debug why a route or selector is counted as covered.
- Use `tests [test-file] --json` to inspect which routes, fetch APIs, test IDs,
  and HTML IDs each Playwright test covers.
- Add `--assert-unique-test-ids` in CI when exact test ID values must be unique
  across the configured app, not just within one linted file. Add
  `--assert-unique-html-ids` when HTML `id` values must also be unique.

## Fix Patterns

- Uncovered route: add Playwright navigation or URL assertion for a URL that
  matches the reported route pattern.
- Uncovered selector: assert the hook with `getByTestId(...)` or a supported CSS
  attribute selector.
- Unsupported dynamic selector: replace it with a literal or static template
  when stable test coverage is expected.
- Duplicate selector: rename exact hook values so each maps to one element or
  state.
- Conditional/skipped test mismatch: use `--assert-conditional-tests` to require
  active tests only, or `--allow-skipped-tests` when skipped tests should count.

## References

If the repository includes this library's docs, read only the reference needed
for the task:

- `docs/cli-reference.md` for flags, config, globs, route matching, and JSON.
- `docs/ast-analysis.md` for supported static AST forms.
- `docs/agent-guide.md` for AGENTS.md or CLAUDE.md instructions.
- `docs/eslint-plugin.md` for ESLint and Oxlint rule setup.
