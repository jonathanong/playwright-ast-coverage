# `no-mistakes dependencies` — full reference

Find every file that the given file(s) transitively import.

## When to use a different tool

- Need a file's exported/imported **symbol list** (its public API) → `no-mistakes symbols`, not `no-mistakes dependencies`.
- Need files that **depend on** a file (reverse direction) → `no-mistakes dependents`, not `no-mistakes dependencies`.
- Need **line-level use sites** of a specific symbol → use `rg` on the result set.
- Need to look at a **workspace package** as a whole (entrypoints, cross-package consumers) → run `no-mistakes dependencies` on the package's entrypoint files (typically `index.mts`).

## Usage

```
no-mistakes dependencies <FILE>... [--root <PATH>] [--tsconfig <FILE>] [--depth <N>]
             [--filter <GLOB>]... [--test <FRAMEWORK>]...
             [--relationship <KIND>]...
             [--format <FORMAT>] [--json] [-j <N>]
```

## How to invoke

```bash
# Find all transitive imports of a file (JSON output — pipe or non-TTY)
no-mistakes dependencies src/main.mts --root /path/to/project

# Limit to direct imports only
no-mistakes dependencies src/main.mts --root /path/to/project --depth 1

# Only include test files in the result
no-mistakes dependencies src/main.mts --root /path/to/project --filter '**/*.test.mts'

# Use --test shorthand for well-known test globs
no-mistakes dependencies src/main.mts --root /path/to/project --test vitest

# Follow only import edges (skip test/route/queue/md/ci/workspace edges)
no-mistakes dependencies src/main.mts --root /path/to/project --relationship import

# Collapse results to folder level
no-mistakes dependencies src/main.mts --root /path/to/project --filter 'backend/services/*/'

# Output as Markdown
no-mistakes dependencies src/main.mts --root /path/to/project --format md

# Explicit tsconfig (otherwise searches upward from --root)
no-mistakes dependencies src/main.mts --root /path/to/project --tsconfig tsconfig.base.json
```

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--root <PATH>` | cwd | Project root |
| `--tsconfig <FILE>` | auto-detected | Path to tsconfig.json |
| `--depth <N>` | unlimited | Max traversal depth |
| `--filter <GLOB>` | none | Include only matching files (repeatable, OR) |
| `--test <FRAMEWORK>` | none | Expand to well-known test globs: `vitest`, `playwright`, `cargo` (repeatable) |
| `--relationship <KIND>` | all | Follow only edges of this kind (repeatable, OR). Values: `import`, `workspace`, `test`, `route`, `queue`, `md`, `ci`, `http`, `process`, `all` |
| `--format <FORMAT>` | human (TTY) / json (pipe) | Output format: `json`, `md`, `yml`, `paths`, `human` |
| `--json` | false | Shorthand for `--format json` |
| `-j / --jobs <N>` | all cores | Worker threads. `0` or omitted = all cores. Honors `RAYON_NUM_THREADS`. |

Discovery is git-aware: tracked files plus untracked non-ignored files are considered, and `.gitignore`d files are skipped. `--relationship import` uses a lazy traversal that parses only reachable static imports. Invalid relationship values fail at argument parsing.

## Output

JSON (`--format json` or `--json` or non-TTY):
```json
{
  "roots": ["src/main.mts"],
  "files": [
    { "path": "src/utils.mts", "depth": 1, "via": ["import"] }
  ]
}
```

Paths (piped default, for shell `$()`):
```
src/utils.mts
src/helpers.mts
```

Human (TTY default):
```
src/main.mts
  src/utils.mts
    src/helpers.mts
```

- `path` is relative to `--root`
- `depth` starts at 1
- `via` lists edge kinds that reached each file (omitted for pure import traversals)

## Notes

- Bare npm specifiers (`express`, `node:path`) are silently ignored
- Static imports/re-exports, type-only imports/references, string-literal dynamic `import()`, and string-literal `require()` are tracked under `--relationship import`
- Route/queue edges are only active when `.guardrailsrc.yml` defines the relevant config
- Patterns ending in `/` in `--filter` collapse results to that folder level
- `#SYMBOL` syntax is NOT supported for `no-mistakes dependencies` (only for `no-mistakes dependents`)
