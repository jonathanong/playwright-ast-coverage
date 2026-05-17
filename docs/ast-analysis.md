# AST Analysis Behavior

The tools parse source files and configuration files statically. They do not run
project code, evaluate build output, query databases, or infer dynamic values
from runtime behavior.

## Shared File Model

- TS/JS source extensions: `.mts`, `.ts`, `.tsx`, `.mjs`, `.js`, `.jsx`, plus
  `.cts` and `.cjs` where a tool explicitly supports them.
- Ignored directories include `.git`, `node_modules`, `target`, `dist`, `build`,
  `coverage`, and test output directories.
- Most outputs are root-relative paths. Inputs may be root-relative or absolute.
- Static literals and expression-free template literals are preferred. Dynamic
  expressions are either skipped, reported as unsupported, or treated as fuzzy
  matches depending on the analyzer.

## Module Graphs

`no-mistakes dependencies`, `dependents`, `related`, and `symbols` parse TS/JS
imports, exports, and package metadata.

Supported import edges:

- Static `import` and `export ... from` declarations.
- Type-only imports and inline `import { type X }` declarations.
- String-literal dynamic `import("...")`.
- String-literal CommonJS `require("...")`.
- Workspace package imports resolved from `package.json#workspaces`.

Resolution support:

- Relative imports with extension fallback.
- `compilerOptions.paths`, including `tsconfig.extends` chains. Path
  replacements are resolved relative to `baseUrl` when it is present, matching
  TypeScript behavior; otherwise they are resolved relative to the tsconfig that
  defines `paths`.
- Workspace package entrypoints and exact or single-`*` export subpaths.

Intentional limits:

- Bare external packages such as `react`, `express`, and `node:path` are ignored.
- `baseUrl`-only aliases are not resolved unless represented in `paths`.
- Non-literal `import()`, `require()`, and computed specifiers are not resolved.
- Symbol queries answer import/export relationships, not line-level call sites.
  Use `rg` on returned files for exact call locations.

## Playwright Coverage

`playwright-ast-coverage` scans Next.js App Router pages and Playwright tests.

Route files are collected under `frontendRoot` from:

- `page.ts`
- `page.tsx`
- `page.js`
- `page.jsx`

Route groups like `(admin)` and parallel route segments like `@modal` do not
contribute URL segments. Dynamic route segments map to `:name`, catch-all
segments map to `*`, and optional catch-all segments map to `**`.

Detected test URL forms include:

```ts
await page.goto("/users/42");
await page.goto("http://localhost:3000/users/42");
await page.click('a[href="/settings"]');
await expect(page).toHaveURL("/settings");
await expect(page).toHaveURL(new RegExp(`/users/${id}`));
await navigateTo(page, "/settings");
```

Absolute URLs count only when they match a literal Playwright `baseURL`.
Negative `.not.toHaveURL(...)` assertions are ignored. Conditional and skipped
tests are tracked with policy flags described in the [CLI reference](cli-reference.md).

Selector coverage collects configured JSX attributes such as `data-testid`,
`data-pw`, mapped component props, and optionally HTML `id` values. Tests cover
them through `getByTestId(...)`, CSS attribute selectors, and CSS ID selectors
when HTML IDs are enabled.

Unsupported dynamic app selectors, such as `data-testid={id}`, are reported but
do not count as covered. Static templates such as `` user-${id} `` are fuzzy and
can be covered by matching static parts.

## Next.js Fetch Calls

`next-to-fetch` maps route, layout, and template files to reachable `fetch()`
calls through static import traversal.

Detected fetch forms include literal and expression-free template URL arguments:

```ts
fetch("/api/users");
fetch(`/api/users`);
fetch("/api/users", { method: "POST" });
```

The report records method, path, route, file, line, client/server side, React
Server Component context, duplicate calls, unsupported dynamic paths, and cache
signals such as `fetch` cache options and known cache wrappers.

Dynamic paths such as `` fetch(`/api/${id}`) `` are reported as unsupported
instead of guessed.

## Queue Graphs

`queue-ast-hop` and `no-mistakes queues` detect static BullMQ and glide-mq
producer/worker relationships.

The graph uses virtual queue-job nodes such as `queues.ts#sendWelcome` so a
producer can connect to the worker that processes the same static job name.
`check` reports unmatched static producers and workers.

Static queue names, job names, queue factory imports, and worker registrations
are required. Dynamic queue or job construction is skipped or reported as
unmatched rather than guessed.

## Server Route Graphs

`server-ast-routes` and `no-mistakes server` extract route definitions and edges
from Node.js server frameworks.

Supported frameworks include Express, Hono, Koa router patterns, and known
project helper shapes. The analyzer records method, normalized route pattern,
source file, and route edges. Dynamic route paths are skipped because guessing
would create noisy graph edges.

## React Traits

`react-traits` and `no-mistakes react` scan React component files and report
traits such as state, props, memoization, environment directives, fetch usage,
and rendered child components.

Component detection is heuristic. It favors static exports, local declarations,
and JSX component usage. Broken files produce parse errors; unknown dynamic
component references are not expanded.

## Lint Plugins

The ESLint/Oxlint plugins enforce code shapes the AST tools can understand:

- `eslint-plugin-playwright-ast-coverage` keeps test IDs literal, unique,
  defaulted, consistently named, and easy to assert with Playwright.
- `eslint-plugin-next-to-fetch` keeps `fetch()` URL and method arguments static.

See [ESLint and Oxlint plugins](eslint-plugin.md).
