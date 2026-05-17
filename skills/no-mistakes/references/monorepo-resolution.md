# Monorepo resolution

The `no-mistakes dependencies`, `no-mistakes dependents`, and `no-mistakes symbols` binaries resolve imports using two mechanisms:

## 1. tsconfig path aliases

When a `tsconfig.json` is present, the binaries load `compilerOptions.paths` and apply them in longest-match-first order.

```json
{
  "compilerOptions": {
    "paths": {
      "@services/*": ["./backend/services/*"],
      "@shared/*": ["./shared/*"]
    }
  }
}
```

**Auto-discovery:** walks upward from `--root` until a `tsconfig.json` is found. In a monorepo with one tsconfig per package and no root tsconfig, auto-discovery often picks the wrong one — specify explicitly:

```bash
no-mistakes dependents backend/services/auth.mts --root /project --tsconfig backend/tsconfig.json
```

**`tsconfig.extends` is followed:** if a workspace tsconfig extends a base config that defines `paths`, the inherited aliases resolve correctly. Pass `--tsconfig` to whichever config in the extends chain contains the relevant `paths` entries, or let auto-discovery find the nearest one.

## 2. npm workspace packages

The binaries load the root `package.json#workspaces` field (array or `{ packages }` object) and build a workspace map:

```json
{
  "workspaces": ["packages/*", "apps/*"]
}
```

Each workspace directory's `package.json` is read for `name`, `exports`, `module`, `main`, and `types`. When an import matches a workspace package name, it resolves to that package's entry point.

**Resolution chain:** `exports["."][import]` → `exports["."][default]` → `module` → `main` → `types` → `src/index.mts` → `index.mts`.

**Known limitation:** only the `"."` entry of `exports` is resolved. Subpath exports like `"./utils"` are not — an import of `@scope/pkg/utils` will not create a graph edge.

## Extension fallback

When a relative import has no extension, the resolver tries:
`.mts` → `.ts` → `.tsx` → `.mjs` → `.js` → `.jsx`

For directory imports (no file suffix), it appends `/index.<ext>` in the same order.

## Resolution priority

1. Relative path with extension fallback
2. tsconfig `paths` alias (longest match first)
3. npm workspace package by `name`
4. Bare specifier → silently dropped (not a graph edge)

## Common patterns

**Per-package tsconfig with aliases:**
```bash
# Use the package's own tsconfig so its aliases resolve
no-mistakes dependents backend/services/auth.mts \
  --root /project \
  --tsconfig backend/tsconfig.json
```

**Multiple tsconfigs, single traversal:**
The tool only loads one tsconfig per run. If your traversal crosses packages with different `paths`, aliases from the secondary package won't resolve. Run separate invocations per package, or use the workspace mechanism for cross-package imports.

**Workspace entrypoints:**
```bash
# Who imports the @scope/core package (via workspace)?
no-mistakes dependents packages/core/src/index.mts --root /project --relationship workspace
```
