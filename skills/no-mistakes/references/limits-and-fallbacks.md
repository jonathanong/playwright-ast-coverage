# Limits and fallbacks

These patterns need extra care when using the module-graph tools. When you hit an unsupported shape, fall back to file-search.

## baseUrl-only imports

`compilerOptions.baseUrl` resolves bare specifiers not listed in `paths`. The tool only reads `paths`, not `baseUrl`.

```json
{
  "compilerOptions": {
    "baseUrl": "./src"
    // import 'utils' resolves to './src/utils.ts' via baseUrl — NOT supported
  }
}
```

**Workaround:** use `rg 'from .utils.' src/` — these imports still appear as literal strings.

## Dynamic import()

String-literal `import("...")` expressions are tracked as `dynamic-import` edges under `--relationship import`. Non-literal expressions are not resolved.

```ts
const mod = await import('./heavy-module.mts');  // tracked
const other = await import(moduleName);          // NOT tracked
```

**Workaround for non-literals:** `rg "import\\(" src/` to find call sites.

## CJS require()

String-literal `require("...")` calls are tracked as `require` edges under `--relationship import`. Non-literal calls are not resolved.

**Workaround for non-literals:** `rg "require(" src/` to find call sites.

## package.json#exports subpaths

For workspace packages, exact subpath entries and single-`*` patterns are resolved. More complex export maps are not.

```json
{
  "exports": {
    ".": "./src/index.mts",
    "./utils": "./src/utils.mts",
    "./*": "./src/*.mts"
  }
}
```

**Workaround for complex export maps:** `rg '@scope/pkg/'` to find importers.

## Bare npm specifiers

Imports of non-workspace npm packages (`express`, `node:path`, `react`) are silently dropped.

This is usually fine — external packages are not project files. If you need to find all consumers of a specific external package, use `rg 'from .express.'`.

## Non-TS/JS files in the graph

The graph only traverses `.mts`, `.ts`, `.tsx`, `.mjs`, `.js`, `.jsx` files for import edges. Other file types (Go, Rust, Python, CSS, JSON, Markdown source files) are not walked.

Exception: Markdown files, CI YAML workflows, and process spawn configs participate via their own edge kinds (`md`, `ci`, `process`) but are not walked for import-style edges.

**Workaround:** file-search (`rg`, `sg`) for non-TS/JS analysis.

## Inline type qualifiers

`import { type X } from "./types"` is tracked as a `type-import` edge. Mixed imports such as `import { type X, Y }` are tracked as regular `import` edges because the module contributes a value binding.

## Namespace imports and symbol queries

`import * as ns from '...'` matches ALL no-mistakes symbols in a `no-mistakes dependents <file>#SYMBOL` query. If you need to verify a specific symbol is actually used (not just the namespace), search the callers manually with `rg`.
