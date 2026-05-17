# CLI Reference

All commands are local and deterministic. Use `--format json` for tooling,
`--format paths` for shell pipelines, and `--timings` where available when an
agent needs to explain cost. `--json` is a shorthand for JSON on commands that
support it.

## `no-mistakes`

Unified entry point for codebase graph and check commands.

```sh
no-mistakes dependencies <FILE>... [--root <PATH>] [--tsconfig <FILE>]
no-mistakes dependents <FILE[#SYMBOL]>... [--root <PATH>] [--tsconfig <FILE>]
no-mistakes related <FILE[#SYMBOL]>...
no-mistakes symbols <FILE>...
no-mistakes react analyze [TARGETS]...
no-mistakes react check [TARGETS]... [--assert-no-fetch]
no-mistakes queues edges [FILES]... [--depth N]
no-mistakes queues related <FILES>... [--direction deps|dependents|both]
no-mistakes queues check
no-mistakes server routes [FILES]...
no-mistakes server edges [ROOTS]... [--depth N]
no-mistakes server related <ROOTS>... [--direction deps|dependents|both]
no-mistakes check
```

### Graph Commands

`dependencies`, `dependents`, and `related` share these options:

| Option | Description |
| --- | --- |
| `--root <PATH>` | Project root. Defaults to the current working directory. |
| `--tsconfig <FILE>` | tsconfig for path aliases. If omitted, searches upward from root. |
| `--depth <N>` | Maximum traversal depth. `--max-depth` is an alias on graph, queue, and server edge commands. Queue/server `edges` default to direct edges when roots are provided, and to the full edge list otherwise. |
| `--filter <GLOB>` | Include only matching files. Repeatable; trailing `/` collapses to folder level. |
| `--test <FRAMEWORK>` | Filter to `vitest`, `playwright`, or `cargo` test globs. Repeatable. |
| `--relationship <KIND>` | Follow only `import`, `workspace`, `test`, `route`, `queue`, `md`, `ci`, `http`, `process`, or `all`. Repeatable. |
| `--format <FORMAT>` | `json`, `md`, `yml`, `paths`, or `human`. |
| `--json` | Shorthand for `--format json`. |
| `--timings` | Emit phase timings on stderr. |
| `-j, --jobs <N>` | Rayon worker threads. `0` means all cores. |

Examples:

```sh
no-mistakes dependencies src/main.mts --relationship import --format json
no-mistakes dependents src/utils.mts --test vitest --format paths
no-mistakes dependents src/queues.mts#sendEmail --json
no-mistakes related web/app/users/page.tsx --relationship test --format paths
```

`FILE#SYMBOL` is supported only by `dependents`/`related`. It finds files that
import that named export, including through re-export chains. Namespace imports
match all symbols.

### Symbols

```sh
no-mistakes symbols src/api.mts --include both --format json
no-mistakes symbols src/types.mts --kind type --kind interface
```

Options: `--root`, `--tsconfig`, repeatable `--kind`, `--include
exports|imports|both`, `--format`, `--json`, `--timings`, and `--jobs`.

### React

```sh
no-mistakes react analyze 'app/components/**/*.tsx' --format json
no-mistakes react check 'app/components/**/*.tsx' --assert-no-fetch
```

Options: `--root`, `--config`, `--format`, and `--json`. `--jobs` is a global
wrapper option, for example `no-mistakes --jobs 4 react ...`.

### Queues

```sh
no-mistakes queues edges --format json
no-mistakes queues edges backend/jobs/email.ts --depth 1
no-mistakes queues related backend/jobs/email.ts --direction dependents --format paths
no-mistakes queues check
```

Options: `--root`, `--tsconfig`, repeatable `--filter`, `--depth` for `edges`,
`--max-depth` as a `--depth` alias, `--format`, `--json`, and `--timings`.
When `edges` receives roots and no depth, it returns direct edges only. `--jobs`
is a global wrapper option, for example `no-mistakes --jobs 4 queues ...`.

### Server Routes

```sh
no-mistakes server routes --format json
no-mistakes server edges backend/api/users.ts --depth 1
no-mistakes server related backend/api/users.ts --direction deps --format paths
```

Options: `--root`, `--tsconfig`, repeatable `--filter`, `--depth` for `edges`,
`--max-depth` as a `--depth` alias, `--format`, `--json`, and `--timings`.
When `edges` receives roots and no depth, it returns direct edges only. `--jobs`
is a global wrapper option, for example `no-mistakes --jobs 4 server ...`.

### Global Check

```sh
no-mistakes check --format json
no-mistakes check --json
```

Runs configured React and queue checks. Options: `--root`, `--config`,
`--tsconfig`, `--format`, and `--json`. `--jobs` is a global wrapper option,
for example `no-mistakes --jobs 4 check ...`.

## `playwright-ast-coverage`

Static Playwright coverage for Next.js App Router routes, selectors, and fetch
assertions.

```sh
playwright-ast-coverage check [OPTIONS]
playwright-ast-coverage edges [OPTIONS]
playwright-ast-coverage related [OPTIONS] <FILES>...
playwright-ast-coverage tests [OPTIONS] [FILES]...
```

| Option | Description |
| --- | --- |
| `--root <ROOT>` | Repository or package root. |
| `--config <CONFIG>` | Analyzer config file. |
| `--playwright-config <FILE>` | Playwright config file. Repeatable. |
| `--project <NAME>` | Top-level Playwright config `name` filter. |
| `--json` | Emit JSON. |
| `--assert-conditional-tests` | Require active test coverage; conditional tests do not count. |
| `--allow-skipped-tests` | Allow skipped tests/suites to count. |
| `--assert-unique-test-ids` | Fail on duplicate exact test IDs across selector roots. |
| `--assert-unique-html-ids` | Fail on duplicate exact HTML `id` values. |
| `--assert-unique-selectors` | Deprecated compatibility alias. |

Examples:

```sh
playwright-ast-coverage check --json
playwright-ast-coverage related 'web/app/users/[id]/page.tsx'
playwright-ast-coverage edges --json
playwright-ast-coverage tests tests/e2e/users.spec.ts --json
```

Supported analyzer config files: `.playwright-ast-coverage.yaml`,
`.playwright-ast-coverage.yml`, `.playwright-ast-coverage.json`, and
`.playwright-ast-coverage.jsonc`.

## `next-to-fetch`

Maps Next.js App Router route files to static `fetch()` calls.

```sh
next-to-fetch [--root <ROOT>] [--config <CONFIG>] [--format <FORMAT>] [--json] [TARGETS]...
```

Targets may be routes such as `/users`, route files, or files imported by route
or layout files. Formats are `json`, `yml`, `paths`, `md`, and `human`; `md` and
`human` render the Markdown report.

```sh
next-to-fetch --root web --format json
next-to-fetch --root web /users app/shared/api.ts
```

## Standalone Queue, Server, And React Binaries

The standalone binaries expose the same analyzers as `no-mistakes` subcommands:

```sh
queue-ast-hop edges --json
queue-ast-hop related backend/jobs/email.ts --direction both
queue-ast-hop check

server-ast-routes routes --format json
server-ast-routes edges backend/api/users.ts --depth 1
server-ast-routes related backend/api/users.ts --format paths

react-traits analyze 'app/components/**/*.tsx' --format json
react-traits check 'app/components/**/*.tsx' --assert-no-fetch
```

Prefer the `no-mistakes` wrapper when an agent needs one consistent command
surface. Use standalone binaries when installing only one tool.
