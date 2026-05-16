---
name: no-mistakes
description: Answer structural TS/JS questions deterministically — what a file imports, who imports it, what it exports. Use for transitive dep graphs, blast-radius, test selection, public-API dumps.
allowed-tools: Bash(no-mistakes:*) Bash(rg:*) Read Glob
version: 0.2.0
---

# No Mistakes

Three scoped subcommands answer structural questions about TS/JS module graphs: `no-mistakes dependencies`, `no-mistakes dependents`, `no-mistakes symbols`. They are resolution-correct (tsconfig paths, workspace packages, extension fallback, barrel re-exports) and deterministic. Use them instead of `rg`/`sg` whenever the question is about **what a file imports or who imports it**.

## When to use these tools vs file-search

| Question | Tool |
|----------|------|
| What does **this file** transitively import? | `no-mistakes dependencies <file>` |
| Which files would be affected by touching **this file**? | `no-mistakes dependents <file>` |
| Which files import **this specific named export**? | `no-mistakes dependents <file>#SYMBOL` |
| What does **this file export / import** (its public API)? | `no-mistakes symbols <file>` |
| Which tests should rerun after changing this file? | `no-mistakes dependents <file> --test vitest` |
| Plain-text search (log messages, comments, strings)? | file-search (`rg`) |
| Exact line numbers where a symbol is called? | `rg` on the files `no-mistakes dependents` returns |

## Quick examples

```bash
# All files this module transitively imports (JSON — machine-readable)
no-mistakes dependencies src/main.mts --root /path/to/project --json

# All files that would be affected by touching this module
no-mistakes dependents src/utils.mts --root /path/to/project --json

# Only files that import the `sendEmail` export
no-mistakes dependents src/queues.mts#sendEmail --root /path/to/project --json

# What does this module export?
no-mistakes symbols src/queues.mts --root /path/to/project --json
```

Pass `--timings` when you need phase timings on stderr. Common labels include `search: Nms`, `ingest: Nms`, `parse: Nms`, `analysis: Nms`, and `output: Nms`; some binaries combine phases (e.g. `parse+analysis: Nms`). Stdout is reserved for the selected data format.

Use `no-mistakes <subcommand> --help` for the full flag reference — `references/*.md` covers the most common patterns.

## Monorepo: no root tsconfig.json

When the project has per-package `tsconfig.json` files (no root tsconfig), pass the relevant one explicitly:

```bash
no-mistakes dependents backend/services/auth.mts --root /path/to/project --tsconfig backend/tsconfig.json
```

Auto-discovery walks upward from `--root`. In a monorepo with multiple tsconfigs, the wrong tsconfig may be picked — specify `--tsconfig` explicitly if you get wrong or empty results. Note: `tsconfig.extends` chains are followed, so passing a workspace tsconfig that extends a base config with `paths` will resolve aliases correctly.

## Hard limits

- **`baseUrl`-only imports** are not resolved (only `paths` mappings are).
- **`package.json#exports` subpaths** support exact keys and single-`*` patterns only.
- **Dynamic `import()`** and **CJS `require()`** are tracked only when the specifier is a string literal.
- **Bare npm specifiers** (`express`, `node:path`) are silently ignored.

See `references/limits-and-fallbacks.md` for workarounds.

## References

| File | What it covers |
|------|----------------|
| `references/decision-tree.md` | Routing table: edge cases, relationship flags, output formats |
| `references/dependencies.md` | Full `no-mistakes dependencies` flag reference and JSON schema |
| `references/dependents.md` | Full `no-mistakes dependents` flag reference, `#SYMBOL` semantics |
| `references/symbols.md` | Full `no-mistakes symbols` flag reference and JSON schema |
| `references/monorepo-resolution.md` | tsconfig paths, workspace packages, `--tsconfig` flag |
| `references/limits-and-fallbacks.md` | Unsupported patterns and `rg`/`sg` workarounds |
