## Design principles

Goal: AI-powered AST-based codebase intelligence for AI Agents.

- Determinism + Heuristics over Probabilistic AI
- CPUs are cheaper than GPUs
- Local over Remote
- Stateless across runs — no filesystem caches, databases, or services. In-memory
  per-run memoization and shared fact maps are allowed to avoid duplicate work.
- Rules to keep the AST parsable (e.g. no indirection, no dynamism)
- Reduce Agent token usage
- Allow custom error messages for agents
- Automatically fix when possible
- If a rule is file-specific, make it an eslint/oxlint rule
- 100% test coverage
- Test fixture-based — can't be perfect, but add more tests to improve coverage
- Heuristics — can't be perfect, but we'll try our best

## Context Management

- By default, show minimum output
- When showing errors, explain what the error is, where it is, how to fix it. For `check` rules, explain why this check exists.

## Development

- When finding an error, always create a regression test
- Continuously add test fixtures to `fixtures/**` for cases you find
- Test fixtures live under `fixtures/<category>/<name>/` at the repo root. Do NOT create fixtures inline in test code (no `fs::create_dir_all` / `fs::write` to build a fixture during a test run). Save the files to `fixtures/*` and reference them via the per-crate / per-package fixture helper.
- All shared Rust code belongs in `no-mistakes-core`. Crates must not depend on one another directly. If two crates need the same helper, lift it into `no-mistakes-core` first.
