# no-mistakes

Deterministic AST-based codebase intelligence for humans and AI agents.

This repository contains Rust CLIs, npm wrappers, ESLint/Oxlint plugins, and
Codex skills for answering structural questions about TypeScript, JavaScript,
React, Next.js, Playwright, queue, and server-route code without running the
application or calling an AI model.

## Start Here

The canonical documentation lives in [docs/](docs/README.md):

- [Documentation index](docs/README.md)
- [CLI reference](docs/cli-reference.md)
- [AST analysis behavior](docs/ast-analysis.md)
- [Agent guide](docs/agent-guide.md)
- [ESLint and Oxlint plugins](docs/eslint-plugin.md)

## Tools

| Tool | Purpose |
| --- | --- |
| `no-mistakes` | Unified codebase graph, symbols, React, queue, server-route, and check commands. |
| `playwright-ast-coverage` | Static Playwright coverage for Next.js App Router routes and selectors. |
| `next-to-fetch` | Map Next.js routes to static `fetch()` API calls. |
| `queue-ast-hop` | Map BullMQ and glide-mq producers to workers. |
| `server-ast-routes` | Extract Express, Hono, Koa, and related server route graphs. |
| `react-traits` | Report React component traits and rendered component relationships. |
| `eslint-plugin-playwright-ast-coverage` | Keep Playwright test IDs static, unique, and consistent. |
| `eslint-plugin-next-to-fetch` | Keep `fetch()` URLs and methods statically analyzable. |

## Install

Use the published packages where available:

```sh
npm install --save-dev no-mistakes playwright-ast-coverage eslint-plugin-playwright-ast-coverage
```

Or install the Rust binary directly:

```sh
cargo install playwright-ast-coverage
```

For local development from a clone, run workspace binaries with Cargo:

```sh
cargo run -p no-mistakes -- dependents src/utils.mts --format paths
```

## Link Lint

Documentation links are linted with [lychee](https://github.com/lycheeverse/lychee):

```sh
lychee --no-progress --exclude-path '^fixtures/' README.md 'docs/**/*.md' 'skills/**/*.md' 'packages/*/README.md' 'crates/*/README.md' CLAUDE.md
```
