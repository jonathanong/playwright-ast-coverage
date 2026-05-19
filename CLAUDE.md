## Design principles

Goal: AI-powered AST-based codebase intelligence for AI Agents.

- Determinism + Heuristics over Probabilistic AI
- CPUs are cheaper than GPUs
- Local over Remote
- Stateless across runs — no filesystem caches, databases, or services. In-memory
  per-run memoization and shared fact maps are allowed to avoid duplicate work.
- One pass per invocation — discover files once, parse TS/JS once for the
  requested facts, then reuse those facts across graph construction and checks.
- Cache only in memory — parsed facts, resolver lookups, traversal results, and
  forward/reverse dependency maps may be cached during a run, but never persisted.
- Build one canonical graph — relationship features should produce typed edges
  in the shared dependency graph instead of maintaining separate graph shapes.
- Fully parallel, deterministic output — independent file analysis and domain
  checks should use rayon/concurrent data structures, then sort/merge before
  rendering results.
- No hardcoded domain conventions — route roots, HTTP prefixes, queue factories,
  workers, and similar project-specific locations must come from configuration.
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

## Coverage

- Coverage gates must enforce 99% line and function coverage.
- **Never** use `cargo llvm-cov --ignore-filename-regex` to suppress uncovered source files. The only files exempt from coverage are test files (`tests/`, sibling `tests.rs`) and test fixtures (`fixtures/`), which `cargo llvm-cov` already excludes by default.
- If a file cannot be brought to 99%, refactor it (extract logic to a lib, thin the entry point) — do not add an exception.
