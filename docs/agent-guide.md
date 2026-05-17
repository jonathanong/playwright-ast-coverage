# Agent Guide

Use these tools before finishing code changes when static codebase intelligence
can reduce missed tests, hidden dependencies, or fragile dynamic patterns.

## Command Selection

| Agent question | Command |
| --- | --- |
| What files does this file import? | `no-mistakes dependencies <file> --format json` |
| What files are affected by this change? | `no-mistakes dependents <file> --format paths` |
| What tests should run? | `no-mistakes dependents <file> --test vitest --format paths` or `playwright-ast-coverage related <file>` |
| What public API does this file expose? | `no-mistakes symbols <file> --include both --format json` |
| Is this App Router route tested? | `playwright-ast-coverage check --json` |
| Which Playwright tests assert this page/component? | `playwright-ast-coverage related <file> --json` |
| Which test IDs/routes/fetches does a test cover? | `playwright-ast-coverage tests <test-file> --json` |
| Which API calls can this Next.js route make? | `next-to-fetch <route-or-file> --format json` |
| Does this queue job have a worker? | `no-mistakes queues check --format json` |
| What server route file owns this endpoint? | `no-mistakes server routes --format json` |
| Does this component tree call fetch? | `no-mistakes react check <glob> --assert-no-fetch --format json` |

## Recommended Agent Instructions

Add project-specific versions of these instructions to `AGENTS.md`, `CLAUDE.md`,
or the repository's agent guide:

```md
Use no-mistakes for structural TS/JS questions before falling back to grep.
Run no-mistakes dependents <changed-file> --format paths to choose focused tests.
Run playwright-ast-coverage check --json before finishing Next.js App Router or Playwright work.
Use playwright-ast-coverage related <file> to identify Playwright tests for changed pages or selector-bearing components.
Keep test IDs and fetch URLs static unless the project explicitly accepts that the AST tools cannot reason about them.
```

## Pre-Finish Workflows

### TS/JS Module Change

```sh
changed=src/utils.mts
no-mistakes symbols "$changed" --include both --format json
no-mistakes dependents "$changed" --format paths
tests=$(no-mistakes dependents "$changed" --test vitest --format paths)
if [ -n "$tests" ]; then
  printf '%s\n' "$tests" | xargs vitest related
fi
```

Use `rg` after `no-mistakes dependents` when you need exact call lines inside
the affected files.

### Next.js App Router Or Playwright Change

```sh
playwright-ast-coverage check --json
playwright-ast-coverage related 'web/app/users/[id]/page.tsx'
playwright-ast-coverage tests --json
```

Fix uncovered routes by adding navigation or URL assertions. Fix uncovered
selectors by asserting a stable test hook with `getByTestId(...)` or a supported
CSS selector.

### API Or Fetch Change

```sh
next-to-fetch --format json
```

If `next-to-fetch` reports dynamic paths, prefer static route strings or small
static wrappers so agents can reason about route-to-API relationships.
When the project uses `eslint-plugin-next-to-fetch`, run the project's own
ESLint command so local config, ignores, and package boundaries are respected.

### Queue Or Server Route Change

```sh
no-mistakes queues check --format json
no-mistakes queues related backend/jobs/email.ts --format paths
no-mistakes server routes --format json
no-mistakes server related backend/api/users.ts --format paths
```

Root-scoped `edges` commands default to direct edges. Pass a larger `--depth`
when you want more transitive hops, or omit roots when you want the full edge
list.

## Failure Handling

- Empty or surprising dependency results usually mean the wrong `--tsconfig`,
  dynamic imports, unsupported aliases, or external package boundaries.
- Dynamic selectors, fetch URLs, route paths, queue names, and job names should
  be made static when the project expects agent-readable behavior.
- For monorepos with multiple tsconfigs, pass the package tsconfig explicitly.
- Treat parse errors as real blockers unless the file is intentionally outside
  the analyzer's supported language set.

## Output Guidance

- Use `--format json` when another tool or agent needs structured data.
- Use `--format paths` for shell pipelines and focused test commands.
- Use `--format human` for interactive debugging.
- Use `--timings` when investigating slow graph queries.
