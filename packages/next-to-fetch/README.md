# next-to-fetch

Map Next.js App Router routes to static `fetch()` API calls.

```sh
npm install --save-dev next-to-fetch
npx next-to-fetch --format json
npx next-to-fetch /users app/shared/api.ts
```

The analyzer reports route files, API methods and paths, client/server context,
duplicates, unsupported dynamic fetches, and cache signals.

See the [documentation index](../../docs/README.md), [CLI reference](../../docs/cli-reference.md),
and [AST analysis behavior](../../docs/ast-analysis.md).
