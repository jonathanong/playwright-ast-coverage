# Documentation

`no-mistakes` is a set of local, deterministic AST tools for codebase
intelligence. The tools are designed for human maintainers and AI agents that
need reliable answers about imports, exported APIs, test impact, routes,
selectors, fetch calls, queue hops, server routes, and React component traits.

## What To Use

| Goal | Command |
| --- | --- |
| Files imported by a file | `no-mistakes dependencies <file>` |
| Files affected by a change | `no-mistakes dependents <file>` or `no-mistakes related <file>` |
| Files importing a named export | `no-mistakes dependents <file>#Symbol` |
| Exports/imports of a TS/JS module | `no-mistakes symbols <file>` |
| React component traits | `no-mistakes react analyze <glob>` or `react-traits analyze <glob>` |
| Queue producer/worker hops | `no-mistakes queues edges` or `queue-ast-hop edges` |
| Server route graph | `no-mistakes server routes` or `server-ast-routes routes` |
| Global project checks | `no-mistakes check` |
| Portable shell checks | `no-mistakes <script>` with `no-mistakes-scripts` installed |
| Playwright route/selector coverage | `playwright-ast-coverage check` |
| Tests related to a page/component | `playwright-ast-coverage related <file>` |
| Playwright assertions grouped by test | `playwright-ast-coverage tests` |
| Next.js routes to fetch calls | `next-to-fetch` |
| Test ID linting | `eslint-plugin-playwright-ast-coverage` |
| Static fetch linting | `eslint-plugin-next-to-fetch` |

## Documentation Map

- [CLI reference](cli-reference.md) lists commands, flags, output formats, and
  common examples for every binary.
- [AST analysis behavior](ast-analysis.md) describes what static code forms are
  recognized and where heuristics intentionally stop.
- [Agent guide](agent-guide.md) gives command-selection and pre-finish workflows
  for AI agents.
- [ESLint and Oxlint plugins](eslint-plugin.md) documents lint rules that keep
  source code analyzable by the CLIs.

## Design Constraints

- Local filesystem input only: no services, databases, or persistent caches.
  In-memory per-run memoization is allowed to avoid duplicate reads and AST work.
- Deterministic AST extraction plus explicit heuristics.
- Prefer static, literal code forms so agents and CI can reason about behavior.
- JSON and path outputs are intended for automation; human and Markdown outputs
  are intended for review.
- When a rule is file-local, prefer an ESLint/Oxlint rule. When a rule needs a
  project graph, use a CLI check.

## Validation

Run link checks with lychee:

```sh
lychee --no-progress --exclude-path '^fixtures/' README.md 'docs/**/*.md' 'skills/**/*.md' 'packages/*/README.md' 'crates/*/README.md' CLAUDE.md
```
