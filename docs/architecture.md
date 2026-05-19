# Architecture

`no-mistakes` is a local, deterministic codebase-intelligence engine. Its
architecture is optimized for AI agents that need reliable project facts while
spending as few tokens and CPU cycles as possible.

This document codifies the performance architecture behind issues `#126`, `#130`, `#132`, and `#133`.

## Core Decisions

1. One pass per invocation.
2. In-memory caching only.
3. Build one canonical graph.
4. Run independent work fully parallel.

These are constraints, not preferences. New features should fit this model
instead of adding separate scanners, persistent caches, background services, or
serial bottlenecks.

Project-specific relationship discovery must be explicit. Route definitions,
HTTP prefixes, queue factories, workers, and similar domain roots should come
from configuration rather than hardcoded repository conventions.

## Invocation Boundary

Each CLI invocation is self-contained:

1. Resolve root, tsconfig, config, entrypoints, and requested relationships.
2. Discover the visible project files once.
3. Parse eligible TS/JS files once for the facts required by the invocation.
4. Build graph edges from those facts and other shared per-run inputs.
5. Query the graph or shared fact maps.
6. Emit deterministic output.

No state is trusted across invocations. Persistent graph caches, daemons,
databases, and filesystem cache directories are intentionally outside the
architecture. If a future feature needs speedups, prefer reducing the per-run
work or sharing more in-memory facts during that run.

## Current Pipeline Shape

The main graph pipeline is centered in `no-mistakes-core`:

| Stage | Current type/module | Role |
| --- | --- | --- |
| File universe | `GraphFiles` | Holds all visible files, indexable TS/JS files, and the visible path set. |
| TS/JS facts | `TsFactPlan`, `TsFileFacts`, `TsFactMap` | Selects and stores facts extracted from one OXC parse per source file. |
| Import resolution | `ImportResolver` | Resolves relative imports and tsconfig aliases using an invocation-scoped cache. |
| Graph build | `DepGraph`, `GraphBuildPlan` | Builds forward and reverse adjacency maps for selected relationship kinds. |
| Traversal | `deps_of`, `dependents_of`, `related` | Runs BFS over the canonical graph with optional edge and path filters. |

The top-level `check` command also shares precomputed facts across domain
checks and runs those checks through `rayon::join`.

## Single-Pass Fact Extraction

`TsFactPlan` is the contract for source parsing. It should grow as new
domain-specific extraction is needed.

Required direction:

1. Add fields to `TsFactPlan` for every fact family needed by graph edges or
   project checks.
2. Add corresponding fields to `TsFileFacts`.
3. Extract all requested TS/JS facts inside the same OXC parse in
   `collect_file_facts`.
4. Pass `TsFactMap` into graph/check builders instead of letting domains read
   and parse files independently.

Domain modules such as routes, queues, HTTP calls, symbols, imports, process
spawns, and future extractors should be visitors/fact producers or graph edge
producers. They should not be independent full-codebase scanners when their
input can come from the shared fact pass.

Acceptable exceptions:

1. Non-TS/JS inputs such as Markdown, package manifests, and CI YAML may have
   their own lightweight readers.
2. A narrowly requested lazy query may read only the reachable frontier if it
   avoids a full graph build and does not duplicate work inside that query.
3. Shared source text may be read once into an invocation-local collection when
   more than one non-AST extractor needs raw text.

## In-Memory Caching

Caches are scoped to one process invocation.

Allowed cache shapes:

1. `TsFactMap`: path to parsed source facts.
2. `GraphFiles.visible`: path membership for resolver and graph checks.
3. `DepGraph.forward` and `DepGraph.reverse`: the dependency tree in both
   directions.
4. `ImportResolver` cache: resolved import specifier lookups.
5. Local traversal caches for expensive per-root searches.
6. Shared raw source buffers when multiple extractors need text.

Disallowed cache shapes:

1. Disk-backed graph caches.
2. Daemons that keep project state between CLI runs.
3. Databases or services.
4. Global mutable caches that survive unrelated invocations.

Parallel caches must not serialize the hot path. Use concurrent maps such as
`DashMap`, per-thread collections followed by deterministic merge, or immutable
shared maps. Do not put `Mutex<HashMap<_, _>>` around a high-frequency cache
used from `rayon` workers.

## Canonical Graph

`DepGraph` is the canonical relationship substrate.

Nodes are:

1. Source files.
2. Virtual nodes when the relationship is real but not a file, such as queue
   jobs.

Edges are typed with `EdgeKind`. The current graph supports imports, type
imports, dynamic imports, requires, workspace imports, test relationships,
routes, queues, Playwright route tests, Markdown links, CI invocations, HTTP
calls, and process spawns.

Every relationship feature should produce edges into this graph unless there is
a strong reason it cannot be represented as a source-to-target relationship.
Queries should prefer graph traversal over bespoke recursive search.

The graph stores both directions:

1. `forward`: node to dependencies.
2. `reverse`: node to dependents.

This double-indexed shape is required. It makes `dependencies`, `dependents`,
`related`, and focused test selection cheap after the build phase.

## Parallel Execution Model

Parallelism is expected at every independent layer:

1. Per-file fact collection uses `rayon` over files.
2. Edge producers use `par_iter` or `into_par_iter` when each file can be
   analyzed independently.
3. Top-level domain checks run concurrently when they consume shared facts.
4. Traversal pre-computation can run per root when roots are independent.
5. Expensive caches must be concurrent or thread-local plus merged.

The CLI initializes the global rayon pool from `--jobs`,
`NO_MISTAKES_JOBS`, or CPU defaults. New commands should use the shared
initialization path instead of creating ad hoc thread pools.

Parallel code must still produce deterministic output:

1. Collect unordered work into vectors or maps.
2. Merge on the main thread when mutation order matters.
3. Sort adjacency lists and output entries before rendering.
4. Keep diagnostics stable and path-based.

## Graph Build Plan

`GraphBuildPlan` prevents unnecessary work. It should mirror the relationship
filters exposed by the CLI:

1. Enable only the edge producers needed by the requested relationships.
2. Reuse the same discovered file universe for every producer.
3. Reuse `TsFactMap` when any enabled producer needs parsed TS/JS facts.
4. Avoid domain-specific rediscovery.

Adding a relationship kind requires updating:

1. `EdgeKind`.
2. `GraphBuildPlan`.
3. CLI relationship filtering.
4. Edge production from shared facts or shared file contents.
5. Fixture-backed graph tests.
6. Documentation for supported and unsupported static forms.

## Extension Checklist

When adding a new analyzer:

1. Decide whether it is file-local, fact-based, graph-based, or output-only.
2. If it is file-local, prefer an ESLint/Oxlint rule.
3. If it needs project context, put shared logic in `no-mistakes-core`.
4. Extend `TsFactPlan` and `TsFileFacts` instead of adding a second TS/JS parse.
5. Add a graph edge when the result is a relationship.
6. Run extraction over files in parallel.
7. Use in-memory caches only.
8. Sort outputs for determinism.
9. Add fixture-based regression tests.

## Anti-Patterns

Avoid these patterns:

1. A new command that independently walks the repository when `GraphFiles` can
   be shared.
2. A domain module that parses every TS/JS file again after `TsFactMap` exists.
3. A high-contention `Mutex<HashMap<_, _>>` inside a `par_iter` loop.
4. A persistent cache that makes results depend on previous runs.
5. A graph-like feature that keeps its own untyped adjacency structure instead
   of adding `EdgeKind` edges.
6. Parallel collection without deterministic sorting before output.

## Implementation Status

Already aligned:

1. `ImportResolver` uses `DashMap` instead of a single locked `HashMap`.
2. `TsFactMap` exists for imports and symbols.
3. `DepGraph` stores forward and reverse maps.
4. Many file and edge collectors use `rayon`.
5. The top-level `check` command shares facts and parallelizes domain checks.

Still to converge:

1. Expand `TsFactPlan` and `TsFileFacts` for route, queue, HTTP, process, and
   other domain-specific facts that currently read source independently.
2. Make graph construction consume the unified fact map wherever possible.
3. Keep lazy query paths explicit so they do not become hidden duplicate parse
   passes.
4. Continue replacing serial shared mutation with concurrent caches or
   thread-local collection plus deterministic merge.
