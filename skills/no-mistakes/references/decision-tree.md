# Decision tree: which tool, which flag?

## Choosing a binary

```
What are you trying to find?
│
├─ Files this file imports (forward graph)
│   └─ no-mistakes dependencies <file> [--depth N] [--relationship import]
│
├─ Files that import this file (reverse graph)
│   └─ no-mistakes dependents <file> [--depth N] [--relationship import]
│
├─ Files that import a specific named export
│   └─ no-mistakes dependents <file>#SYMBOL
│
├─ Named exports of a file (public API)
│   └─ no-mistakes symbols <file>
│
├─ Named imports of a file (what it consumes)
│   └─ no-mistakes symbols <file> --include imports
│
├─ React component traits / fetch checks
│   └─ no-mistakes react analyze 'app/components/**/*.tsx'
│   └─ no-mistakes react check 'app/components/**/*.tsx' --assert-no-fetch
│
├─ Queue producer/worker hops
│   └─ no-mistakes queues edges [file] [--depth N]
│   └─ no-mistakes queues check
│
├─ Server route extraction / related files
│   └─ no-mistakes server routes
│   └─ no-mistakes server related <file>
│
├─ Tests to run after changing a file
│   └─ no-mistakes dependents <file> --test vitest --relationship test
│   or no-mistakes dependents <file> --test playwright --relationship test
│
├─ Which routes or queue jobs reach a file
│   └─ no-mistakes dependents <file> --relationship route
│   └─ no-mistakes dependents <file> --relationship queue
│   (requires .guardrailsrc.yml with the relevant rule configured)
│
└─ Which CI workflows invoke a binary
    └─ no-mistakes dependents src/bin/mybinary.rs --relationship ci
```

## Choosing a --relationship flag

| Flag value | What edges it follows |
|---|---|
| `import` | Static TS/JS imports and `import type` |
| `workspace` | Cross-package npm workspace imports and `package.json` workspace no-mistakes dependencies |
| `test` | `foo.mts` ↔ `foo.test.mts` test correspondence |
| `route` | Route reference → route definition |
| `queue` | Queue enqueue/worker relationship → virtual queue job |
| `md` | Markdown link → linked source file |
| `ci` | CI workflow YAML → binary entry point |
| `http` | HTTP client call with a static path → backend route definition |
| `process` | `spawn`/`exec`/Playwright `webServer` → spawned entry file |
| `all` | All of the above (default) |

Repeatable — `--relationship import --relationship workspace` follows both kinds.

## Output format selection

| Format | When to use |
|---|---|
| `--format json` / `--json` | Feeding to another tool, agent parsing |
| `--format paths` | Shell `$()` substitution, xargs |
| `--format md` | Writing to a document |
| `--format human` | Debugging interactively on a TTY |
| `--format yml` | YAML pipelines |

Default: `human` on TTY, `json` when piped.

## Filtering results

```bash
# Only files matching a glob
no-mistakes dependents src/auth.mts --filter 'backend/**/*.mts'

# Collapse to folder level (trailing /)
no-mistakes dependents src/auth.mts --filter 'backend/services/*/'

# Combine multiple globs (OR)
no-mistakes dependents src/auth.mts --filter 'backend/**' --filter 'integration-tests/**'
```

## Edge cases

**When to use rg instead of no-mistakes dependents for callers:**
`no-mistakes dependents` answers "who imports this file/symbol" with resolution-correct graph traversal. Use `rg` when you need the specific line of code where a symbol is called, or when a pattern may appear in non-import contexts (template strings, comments, dynamic lookups).

**When to pass --tsconfig explicitly:**
In a monorepo with per-package tsconfigs and no root `tsconfig.json`, auto-discovery may pick the wrong one. Pass `--tsconfig <pkg>/tsconfig.json` whenever you get empty or wrong results from a file inside a specific package.

**When no-mistakes dependents returns fewer results than expected:**
Check if the import uses a bare external specifier, a non-literal dynamic `import()` / `require()`, or an alias that requires a specific package `tsconfig`. See `limits-and-fallbacks.md` for workarounds.
