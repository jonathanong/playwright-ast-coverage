# Test Fixtures

Each subdirectory is a minimal project used by one or more tests. Add to this
file whenever you add a new fixture so orphans are immediately visible.

| Fixture | Owning test(s) |
|---------|----------------|
| `aliased/` | `src/dependencies/graph/tests.rs` — aliased import resolution |
| `codebase-intel/` | `src/dependencies/graph/tests.rs` — NodeId virtual nodes, TypeImport, RouteTest |
| `dependents-basic/` | `tests/cli_dependents.rs` — fixture-only `dependents` traversal, output, and relationship coverage |
| `filaments-workspace/` | `src/workspaces/tests.rs::fixture_filaments_workspace_session_jwt_conditional_exports` |
| `filter/` | `src/dependencies/tests.rs` — `--filter` glob flag |
| `folder-suffix/` | `src/dependencies/tests.rs` — folder-suffix filter |
| `format-output/` | `src/dependencies/tests.rs` — `--format` output modes |
| `guardrails-config/` | `src/guardrails/config/tests.rs::parses_guardrails_config_fixture` |
| `migrations/` | `src/pg_schema/lint/tests.rs` — no-drop-view lint rule |
| `mixed-type-import/` | `src/dependencies/extract/tests.rs::fixture_mixed_type_import_file` |
| `lockfiles/` | `src/guardrails/rules/package_manager_lockfiles/tests.rs` |
| `max-file-size/` | `src/guardrails/rules/max_file_size/tests.rs` |
| `max-test-size/` | `src/guardrails/rules/max_test_size/tests.rs` |
| `mts-extensions-fs/` | `src/guardrails/rules/mts_extensions/tests.rs` |
| `glide-mq-required-backoff-jitter/` | `src/guardrails/rules/glide_mq_required_backoff_jitter/tests.rs` |
| `psql-test-location/` | `src/guardrails/rules/psql_test_files_location/tests.rs` |
| `repo-structure/` | `src/guardrails/rules/repo_structure/tests.rs` |
| `vitest-config/` | `src/guardrails/rules/single_vitest_config/tests.rs` |
| `web-middleware/` | `src/guardrails/rules/web_middleware_to_proxy/tests.rs` |
| `pg-schema-migrations/` | `src/pg_schema/lint/tests.rs`; `tests/cli_pg_schema.rs` |
| `queue-dashboard/` | `src/guardrails/rules/queue_dashboard_reachability/tests.rs`; `src/ts_queues/factory/tests.rs` |
| `queue-usage/` | `src/ts_queues/factory/tests.rs::fixture_queue_usage_extracts_queue_names` |
| `routes/` | `src/guardrails/rules/route_consistency/tests.rs`; `src/ts_routes/defs_frontend/tests.rs` |
| `routes-backend/` | `src/ts_routes/defs_backend/tests.rs::fixture_routes_backend_extracts_real_filaments_pattern` |
| `simple/` | `src/dependencies/graph/tests.rs`; `tests/cli_dependencies.rs`; `tests/cli_guardrails.rs` |
| `systems-worker-required-options/` | `src/guardrails/rules/systems_worker_required_options/tests.rs` |
| *(inline)* | `src/ts_http_calls/tests.rs` — HTTP client call extraction (inline source strings, no fixture dir) |
| *(inline)* | `src/ts_process_spawn/tests.rs` — process spawn extraction (tempdir per test, no fixture dir) |
| `symbol-export/` | `src/dependencies/tests.rs` — `dependents` with symbol filter |
| `test-correspondence/` | `src/guardrails/rules/test_correspondence/tests.rs` |
| `test-framework/` | `src/dependencies/graph/tests.rs`; `src/dependencies/tests.rs` — Vitest/Playwright globs |
