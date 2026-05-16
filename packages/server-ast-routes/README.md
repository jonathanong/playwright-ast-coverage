# server-ast-routes

Map Node.js server route definitions to normalized API route patterns.

```sh
npm install --save-dev server-ast-routes
npx server-ast-routes routes --json
npx server-ast-routes edges backend/api/users.ts --format paths
```

The analyzer supports Express and Hono route definitions natively, with
heuristics for `@jongleberry/api-server`, `@koa/router`, and `koa-path-match`.
Dynamic route paths are skipped rather than guessed.
