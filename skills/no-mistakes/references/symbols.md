# `no-mistakes symbols` — full reference

Find every named export and (optionally) named import of one or more TS/JS files.

## When to use a different tool

- Need files that import this file or symbol → `no-mistakes dependents`, not `no-mistakes symbols`.
- Need files this file imports → `no-mistakes dependencies`, not `no-mistakes symbols`.

## Usage

```
no-mistakes symbols <FILE>... [--root <PATH>] [--tsconfig <FILE>]
        [--kind <KIND>]... [--include exports|imports|both]
        [--format <FORMAT>] [--json] [-j <N>]
```

## How to invoke

```bash
# What does this file export? (JSON in pipes, human tree on TTY)
no-mistakes symbols src/queues.mts --root /path/to/project

# Both exports and imports — full module surface
no-mistakes symbols src/queues.mts --root /path/to/project --include both

# Only the type-level exports
no-mistakes symbols src/types.mts --root /path/to/project --kind type --kind interface --kind enum

# Just functions
no-mistakes symbols src/utils.mts --root /path/to/project --kind function

# Multiple files at once, JSON output
no-mistakes symbols src/a.mts src/b.mts --root /path/to/project --json
```

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--root <PATH>` | cwd | Project root |
| `--tsconfig <FILE>` | auto-detected | Path to tsconfig.json |
| `--kind <KIND>` | all | Only include exports of this kind (repeatable). Values: `function`, `class`, `const`, `let`, `var`, `type`, `interface`, `enum`, `default`, `re-export` |
| `--include <SECTION>` | `exports` | What to emit: `exports`, `imports`, or `both` |
| `--format <FORMAT>` | human (TTY) / json (pipe) | Output format: `json`, `md`, `yml`, `paths`, `human` |
| `--json` | false | Shorthand for `--format json` |
| `-j / --jobs <N>` | all cores | Worker threads. Honors `RAYON_NUM_THREADS`. |

## Output

JSON (`--format json` or `--json` or non-TTY):
```json
{
  "roots": ["src/queues.mts"],
  "files": [{
    "path": "src/queues.mts",
    "exports": [
      { "name": "sendEmail", "kind": "function", "line": 42 },
      { "name": "withRetry", "kind": "re-export", "line": 3,
        "reExport": { "source": "./retry.mts", "imported": "withRetry", "resolved": "src/retry.mts" } }
    ],
    "imports": [
      { "source": "@systems/_shared", "imported": "createQueue", "local": "createQueue",
        "line": 1, "typeOnly": false, "resolved": "packages/_shared/index.mts" }
    ]
  }]
}
```

Paths (one entry per line, `path:line:name` — ripgrep `--vimgrep` style):
```
src/queues.mts:42:sendEmail
src/queues.mts:3:withRetry
```

## Notes

- `path` in JSON/YAML output is relative to `--root`
- For re-exports, `reExport.resolved` is the project-relative path of the source module if resolvable; absent for bare npm specifiers
- For imports, `resolved` is similarly project-relative when resolvable; absent for bare specifiers
- Default `--include` is `exports` because the most common question is "what does this module expose?"
- `--kind` filter only applies to exports, not imports
- TSX/JSX files are auto-detected by extension and parsed with the appropriate grammar
- Only top-level statements are walked — nested or conditional exports are not surfaced
