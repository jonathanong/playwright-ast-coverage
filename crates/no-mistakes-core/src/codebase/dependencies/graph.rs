pub(crate) mod playwright;

use super::extract::{is_indexable, is_tsx_file, ExtractedImport, ImportExtractor, ImportKind};
use crate::codebase::ts_resolver::{ImportResolver, TsConfig};
use crate::codebase::ts_source::facts::{
    collect_ts_facts, collect_ts_facts_with_context, TsFactContext, TsFactMap, TsFactPlan,
};
use crate::codebase::ts_symbols::ExportKind;
use anyhow::Result;
use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

pub use crate::codebase::ts_source::SKIP_DIRS;

/// A node in the dependency graph: either a source file or a virtual queue-job node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeId {
    /// A source file on disk.
    File(PathBuf),
    /// A virtual job node representing one (queue, jobName) pair.
    QueueJob { queue_file: PathBuf, job: String },
}

impl NodeId {
    /// Return the underlying file path, if this is a `File` node.
    pub fn as_file(&self) -> Option<&Path> {
        match self {
            NodeId::File(p) => Some(p.as_path()),
            NodeId::QueueJob { .. } => None,
        }
    }

    /// Render this node relative to `root` for display.
    pub fn display_name(&self, root: &Path) -> String {
        match self {
            NodeId::File(p) => {
                let rel = p.strip_prefix(root).unwrap_or(p);
                rel.display().to_string()
            }
            NodeId::QueueJob { queue_file, job } => {
                let rel = queue_file
                    .strip_prefix(root)
                    .unwrap_or(queue_file.as_path());
                format!("{}#{}", rel.display(), job)
            }
        }
    }
}

/// The kind of dependency edge connecting two nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EdgeKind {
    /// Regular TS/JS static import.
    Import,
    /// Type-only import (`import type ...`).
    TypeImport,
    /// Runtime dynamic import (`import("...")`).
    DynamicImport,
    /// CommonJS `require("...")` call.
    Require,
    /// Test correspondence: `foo.mts` ↔ `foo.test.mts`.
    TestOf,
    /// Frontend/backend route reference: ref_file → route_def_file.
    RouteRef,
    /// Enqueue site → QueueJob virtual node.
    QueueEnqueue,
    /// QueueJob virtual node → worker/processor file.
    QueueWorker,
    /// Playwright test ↔ frontend page file.
    RouteTest,
    /// Markdown link: `*.md` → linked file.
    MarkdownLink,
    /// Cross-workspace package import (via npm workspace resolution).
    WorkspaceImport,
    /// CI workflow invokes a binary: `*.yml` → `src/bin/*.rs`.
    CiInvocation,
    /// HTTP call from a client file to a backend route-definition file.
    HttpCall,
    /// Process spawn: a file launches another file via `spawn`/`exec`/playwright webServer.
    ProcessSpawn,
}

/// A single node in the traversal result.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeEntry {
    /// The graph node (file or virtual queue-job).
    pub node: NodeId,
    /// Traversal depth (1 = direct dep/dependent, 2 = transitive, etc.).
    pub depth: usize,
    /// Edge kinds that led to this node (deduped, sorted).
    pub via: Vec<EdgeKind>,
}

type EdgeMap = HashMap<NodeId, Vec<(NodeId, EdgeKind)>>;

// An edge in both directions: (from, to, kind).
type Edge = (NodeId, NodeId, EdgeKind);

type ParsedImports = Vec<(PathBuf, Vec<ExtractedImport>)>;

/// Selects which edge producers run while building a dependency graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GraphBuildPlan {
    pub imports: bool,
    pub workspace: bool,
    pub tests: bool,
    pub markdown: bool,
    pub ci: bool,
    pub routes: bool,
    pub queues: bool,
    pub playwright_routes: bool,
    pub http: bool,
    pub process: bool,
}

impl GraphBuildPlan {
    pub fn all() -> Self {
        Self {
            imports: true,
            workspace: true,
            tests: true,
            markdown: true,
            ci: true,
            routes: true,
            queues: true,
            playwright_routes: true,
            http: true,
            process: true,
        }
    }

    /// Minimal plan for import-only traversal (no routes, queues, http, etc.).
    pub fn imports_and_workspace() -> Self {
        Self {
            imports: true,
            workspace: true,
            ..Self::default()
        }
    }

    pub fn from_allowed(allowed: Option<&HashSet<EdgeKind>>) -> Self {
        let Some(allowed) = allowed else {
            return Self::all();
        };
        Self {
            imports: allowed.contains(&EdgeKind::Import)
                || allowed.contains(&EdgeKind::TypeImport)
                || allowed.contains(&EdgeKind::DynamicImport)
                || allowed.contains(&EdgeKind::Require),
            workspace: allowed.contains(&EdgeKind::WorkspaceImport),
            tests: allowed.contains(&EdgeKind::TestOf),
            markdown: allowed.contains(&EdgeKind::MarkdownLink),
            ci: allowed.contains(&EdgeKind::CiInvocation),
            routes: allowed.contains(&EdgeKind::RouteRef),
            queues: allowed.contains(&EdgeKind::QueueEnqueue)
                || allowed.contains(&EdgeKind::QueueWorker),
            playwright_routes: allowed.contains(&EdgeKind::RouteTest),
            http: allowed.contains(&EdgeKind::HttpCall),
            process: allowed.contains(&EdgeKind::ProcessSpawn),
        }
    }

    pub(crate) fn ts_fact_plan(self) -> TsFactPlan {
        TsFactPlan {
            imports: self.imports || self.workspace,
            symbols: self.queues,
            route_refs: self.routes,
            backend_routes: self.routes || self.http,
            queue_usage: self.queues,
            queue_factory: self.queues,
            http_calls: self.http,
            process_spawns: self.process,
            ..TsFactPlan::default()
        }
    }
}

#[derive(Clone)]
pub(crate) struct GraphFiles {
    all: Vec<PathBuf>,
    indexable: Vec<PathBuf>,
    visible: HashSet<PathBuf>,
}

impl GraphFiles {
    pub(crate) fn discover(root: &Path) -> Self {
        let all = crate::codebase::ts_source::discover_files(root, &[]);
        Self::from_files(all)
    }

    pub(crate) fn from_files(all: Vec<PathBuf>) -> Self {
        let visible = all.iter().cloned().collect();
        let indexable = all.iter().filter(|p| is_indexable(p)).cloned().collect();
        Self {
            all,
            indexable,
            visible,
        }
    }

    fn is_visible(&self, path: &Path) -> bool {
        self.visible.contains(path)
    }

    pub(crate) fn indexable(&self) -> &[PathBuf] {
        &self.indexable
    }

    pub(crate) fn visible(&self) -> &HashSet<PathBuf> {
        &self.visible
    }
}

pub(crate) fn ts_fact_context_for_plan(root: &Path, plan: GraphBuildPlan) -> TsFactContext {
    let options = graph_config_options(root);
    ts_fact_context_from_options(root, plan, options.as_ref())
}

#[derive(Clone)]
struct GraphConfigOptions {
    route: crate::codebase::config::RouteOptions,
    queue: crate::codebase::config::QueueOptions,
    http_route: crate::codebase::config::HttpRouteOptions,
    http_call: crate::codebase::config::HttpCallOptions,
}

fn graph_config_options(root: &Path) -> Option<GraphConfigOptions> {
    let config = crate::codebase::config::load_config(root).ok()?;
    Some(GraphConfigOptions {
        route: config.rule_options("route-consistency"),
        queue: config.rule_options("queue-dashboard-reachability"),
        http_route: config.rule_options("http-route-static-paths"),
        http_call: config.rule_options("http-call-static-paths"),
    })
}

fn ts_fact_context_from_options(
    root: &Path,
    plan: GraphBuildPlan,
    options: Option<&GraphConfigOptions>,
) -> TsFactContext {
    let mut context = TsFactContext::new(root);
    let Some(options) = options else {
        return context;
    };
    if plan.routes || plan.http {
        context.backend_register_object = resolved_backend_register_object(options);
        context.backend_route_glob = resolved_backend_pattern(options)
            .as_deref()
            .and_then(compile_graph_glob);
        context.http_prefixes = resolved_backend_prefixes(options);
    }
    if plan.queues
        && !options.queue.factory_specifier.is_empty()
        && !options.queue.factory_function.is_empty()
    {
        context.queue_factory_specifier = Some(options.queue.factory_specifier.clone());
        context.queue_factory_function = Some(options.queue.factory_function.clone());
        context.queue_factory_glob = compile_graph_glob(&options.queue.queue_pattern);
    }
    context
}

fn compile_graph_glob(pattern: &str) -> Option<GlobSet> {
    if pattern.is_empty() {
        return None;
    }
    let glob = GlobBuilder::new(pattern)
        .literal_separator(false)
        .build()
        .ok()?;
    let mut builder = GlobSetBuilder::new();
    builder.add(glob);
    builder.build().ok()
}

fn resolved_backend_pattern(options: &GraphConfigOptions) -> Option<String> {
    if !options.http_route.backend_pattern.is_empty() {
        Some(options.http_route.backend_pattern.clone())
    } else if !options.route.backend_pattern.is_empty() {
        Some(options.route.backend_pattern.clone())
    } else {
        None
    }
}

fn resolved_backend_register_object(options: &GraphConfigOptions) -> Option<String> {
    if !options.http_route.register_object.is_empty() {
        Some(options.http_route.register_object.clone())
    } else if !options.route.backend_register_object.is_empty() {
        Some(options.route.backend_register_object.clone())
    } else {
        None
    }
}

fn resolved_backend_prefixes(options: &GraphConfigOptions) -> Vec<String> {
    if !options.http_call.backend_prefixes.is_empty() {
        options.http_call.backend_prefixes.clone()
    } else {
        options.route.backend_prefixes.clone()
    }
}

fn add_edge(map: &mut EdgeMap, from: NodeId, to: NodeId, kind: EdgeKind) {
    map.entry(from).or_default().push((to, kind));
}

fn add_file_edge(map: &mut EdgeMap, from: PathBuf, to: PathBuf, kind: EdgeKind) {
    add_edge(map, NodeId::File(from), NodeId::File(to), kind);
}

fn normalize_nodes(nodes: &[NodeId]) -> Vec<NodeId> {
    nodes
        .iter()
        .map(|node| match node {
            NodeId::File(path) => NodeId::File(crate::codebase::ts_resolver::normalize_path(path)),
            NodeId::QueueJob { queue_file, job } => NodeId::QueueJob {
                queue_file: crate::codebase::ts_resolver::normalize_path(queue_file),
                job: job.clone(),
            },
        })
        .collect()
}

/// Merge a flat list of edges into forward and reverse maps.
fn merge_edges(forward: &mut EdgeMap, reverse: &mut EdgeMap, edges: Vec<Edge>) {
    for (from, to, kind) in edges {
        forward
            .entry(from.clone())
            .or_default()
            .push((to.clone(), kind));
        reverse.entry(to).or_default().push((from, kind));
    }
}

/// Directed dependency graph: node → nodes it depends on, and the reverse.
pub struct DepGraph {
    root: PathBuf,
    /// forward: node → nodes it imports/references (with edge kinds)
    forward: EdgeMap,
    /// reverse: node → nodes that import/reference it (with edge kinds)
    reverse: EdgeMap,
}

impl DepGraph {
    pub fn build(root: &Path, tsconfig: &TsConfig) -> Result<Self> {
        Self::build_with_plan(root, tsconfig, GraphBuildPlan::all())
    }

    pub fn build_with_plan(root: &Path, tsconfig: &TsConfig, plan: GraphBuildPlan) -> Result<Self> {
        let graph_files = GraphFiles::discover(root);
        Ok(Self::build_with_plan_and_files(
            root,
            tsconfig,
            plan,
            &graph_files,
        ))
    }

    pub(crate) fn build_with_plan_and_files(
        root: &Path,
        tsconfig: &TsConfig,
        plan: GraphBuildPlan,
        graph_files: &GraphFiles,
    ) -> Self {
        Self::build_with_plan_files_and_facts(root, tsconfig, plan, graph_files, None)
    }

    pub(crate) fn build_with_plan_files_and_facts(
        root: &Path,
        tsconfig: &TsConfig,
        plan: GraphBuildPlan,
        graph_files: &GraphFiles,
        facts: Option<&TsFactMap>,
    ) -> Self {
        let ts_ex = ImportExtractor::for_typescript().expect("typescript import extractor builds");
        let tsx_ex = ImportExtractor::for_tsx().expect("tsx import extractor builds");
        let resolver = ImportResolver::new(tsconfig).with_visible(graph_files.visible());
        let fact_plan = plan.ts_fact_plan();
        let config_options = graph_config_options(root);
        let fact_context = ts_fact_context_from_options(root, plan, config_options.as_ref());
        let owned_facts = if !fact_plan.is_empty() && facts.is_none() {
            Some(collect_ts_facts_with_context(
                graph_files.indexable(),
                fact_plan,
                &fact_context,
            ))
        } else {
            None
        };
        let facts = owned_facts.as_ref().or(facts);

        let mut forward: EdgeMap = HashMap::new();
        let mut reverse: EdgeMap = HashMap::new();

        let files = &graph_files.indexable;

        // Pre-populate all known file nodes.
        for f in files {
            forward.entry(NodeId::File(f.clone())).or_default();
        }

        let parsed_imports = if plan.imports || plan.workspace {
            match facts {
                Some(facts) => collect_parsed_imports_from_facts(files, facts),
                None => collect_parsed_imports(files, &ts_ex, &tsx_ex),
            }
        } else {
            Vec::new()
        };

        if plan.imports {
            let import_edges = collect_import_edges(&parsed_imports, &resolver, graph_files);
            merge_edges(&mut forward, &mut reverse, import_edges);
        }

        if plan.workspace {
            let workspace = crate::codebase::workspaces::load_from_files(root, &graph_files.all)
                .unwrap_or_default();
            let workspace_edges =
                collect_workspace_edges(&parsed_imports, &resolver, &workspace, graph_files);
            merge_edges(&mut forward, &mut reverse, workspace_edges);
            let workspace_manifest_edges =
                collect_workspace_manifest_edges(&graph_files.all, &workspace, graph_files);
            merge_edges(&mut forward, &mut reverse, workspace_manifest_edges);
        }

        if plan.tests {
            let test_edges = collect_test_edges(files);
            merge_edges(&mut forward, &mut reverse, test_edges);
        }

        if plan.markdown {
            let md_edges = collect_md_edges(&graph_files.all, graph_files);
            merge_edges(&mut forward, &mut reverse, md_edges);
        }

        if plan.ci {
            add_ci_edges(root, &graph_files.all, &mut forward, &mut reverse);
        }

        if plan.routes {
            let route_edges = collect_route_edges(
                root,
                tsconfig,
                &graph_files.all,
                facts,
                config_options.as_ref(),
            );
            merge_edges(&mut forward, &mut reverse, route_edges);
        }

        if plan.queues {
            add_queue_edges(root, &resolver, files, facts, &mut forward, &mut reverse);
        }

        if plan.playwright_routes {
            let playwright_edges = collect_playwright_route_edges(root, &graph_files.all);
            merge_edges(&mut forward, &mut reverse, playwright_edges);
        }

        // Read file contents once and share across steps 9 and 10 to avoid
        // redundant disk reads (files are already in OS page cache but the
        // syscall overhead adds up across thousands of files).
        if plan.http || plan.process {
            let file_contents: Vec<(PathBuf, String)> = if facts.is_some() {
                Vec::new()
            } else {
                files
                    .par_iter()
                    .filter_map(|p| std::fs::read_to_string(p).ok().map(|s| (p.clone(), s)))
                    .collect()
            };

            if plan.http {
                let http_call_edges = collect_http_call_edges(
                    root,
                    tsconfig,
                    facts,
                    &file_contents,
                    graph_files.indexable(),
                    &graph_files.all,
                    config_options.as_ref(),
                );
                merge_edges(&mut forward, &mut reverse, http_call_edges);
            }

            if plan.process {
                let spawn_edges = collect_process_spawn_edges(
                    root,
                    facts,
                    &file_contents,
                    graph_files.indexable(),
                );
                merge_edges(&mut forward, &mut reverse, spawn_edges);
            }
        }

        // Sort adjacency lists for deterministic BFS output.
        for adj in forward.values_mut() {
            adj.sort_by_key(|(n, k)| (node_sort_key(n), *k as u8));
        }
        for adj in reverse.values_mut() {
            adj.sort_by_key(|(n, k)| (node_sort_key(n), *k as u8));
        }

        Self {
            root: root.to_path_buf(),
            forward,
            reverse,
        }
    }

    pub(crate) fn build_with_plan_file_list_and_facts(
        root: &Path,
        tsconfig: &TsConfig,
        plan: GraphBuildPlan,
        files: Vec<PathBuf>,
        facts: &TsFactMap,
    ) -> Self {
        let graph_files = GraphFiles::from_files(files);
        Self::build_with_plan_files_and_facts(root, tsconfig, plan, &graph_files, Some(facts))
    }

    /// Construct a graph directly from pre-built maps (for testing).
    #[cfg(test)]
    pub fn from_raw_maps(
        root: PathBuf,
        forward: HashMap<PathBuf, Vec<PathBuf>>,
        reverse: HashMap<PathBuf, Vec<PathBuf>>,
    ) -> Self {
        let typed_fwd: EdgeMap = forward
            .into_iter()
            .map(|(k, vs)| {
                (
                    NodeId::File(k),
                    vs.into_iter()
                        .map(|v| (NodeId::File(v), EdgeKind::Import))
                        .collect(),
                )
            })
            .collect();
        let typed_rev: EdgeMap = reverse
            .into_iter()
            .map(|(k, vs)| {
                (
                    NodeId::File(k),
                    vs.into_iter()
                        .map(|v| (NodeId::File(v), EdgeKind::Import))
                        .collect(),
                )
            })
            .collect();
        Self {
            root,
            forward: typed_fwd,
            reverse: typed_rev,
        }
    }

    /// Find all nodes that `roots` transitively depend on (follow imports).
    pub fn deps_of(
        &self,
        roots: &[NodeId],
        max_depth: Option<usize>,
        allowed: Option<&HashSet<EdgeKind>>,
    ) -> Vec<NodeEntry> {
        let roots = normalize_nodes(roots);
        bfs(&roots, &self.forward, max_depth, allowed)
    }

    /// Find all nodes that transitively reference `roots` (reverse direction).
    pub fn dependents_of(
        &self,
        roots: &[NodeId],
        max_depth: Option<usize>,
        allowed: Option<&HashSet<EdgeKind>>,
    ) -> Vec<NodeEntry> {
        let roots = normalize_nodes(roots);
        bfs(&roots, &self.reverse, max_depth, allowed)
    }

    /// Find all files that import `symbol` from `file`, transitively.
    pub fn dependents_of_symbol(
        &self,
        file: &Path,
        symbol: &str,
        max_depth: Option<usize>,
        allowed: Option<&HashSet<EdgeKind>>,
        symbol_index: &SymbolIndex,
    ) -> Vec<NodeEntry> {
        let mut visited_pairs: HashSet<(PathBuf, String)> = HashSet::new();
        let mut queue: VecDeque<(PathBuf, String)> = VecDeque::new();
        let mut direct_importers: HashSet<NodeId> = HashSet::new();

        let start = (
            crate::codebase::ts_resolver::normalize_path(file),
            symbol.to_string(),
        );
        visited_pairs.insert(start.clone());
        queue.push_back(start);

        while let Some((src_file, sym)) = queue.pop_front() {
            if let Some(importers) = symbol_index.importers_of(&src_file, &sym) {
                for (importer, local_name, is_reexport) in importers {
                    direct_importers.insert(NodeId::File(importer.clone()));
                    if *is_reexport {
                        let pair = (importer.clone(), local_name.clone());
                        push_unvisited_symbol_pair(&mut visited_pairs, &mut queue, pair);
                    }
                }
            }
        }

        // Also check if (file, symbol) corresponds to a QueueJob node.
        let queue_job = NodeId::QueueJob {
            queue_file: file.to_path_buf(),
            job: symbol.to_string(),
        };
        if self.reverse.contains_key(&queue_job) {
            direct_importers.insert(queue_job);
        }

        let roots: Vec<NodeId> = direct_importers.into_iter().collect();
        bfs(&roots, &self.reverse, max_depth, allowed)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn all_files(&self) -> impl Iterator<Item = &NodeId> {
        self.forward.keys()
    }
}

fn push_unvisited_symbol_pair(
    visited_pairs: &mut HashSet<(PathBuf, String)>,
    queue: &mut VecDeque<(PathBuf, String)>,
    pair: (PathBuf, String),
) {
    if visited_pairs.insert(pair.clone()) {
        queue.push_back(pair);
    }
}

/// Demand-driven import traversal used by `dependencies --relationship import`.
/// It parses only roots and files reached through static import edges.
pub fn lazy_import_deps_of(
    roots: &[NodeId],
    root: &Path,
    tsconfig: &TsConfig,
    max_depth: Option<usize>,
) -> Result<Vec<NodeEntry>> {
    let graph_files = GraphFiles::discover(root);
    Ok(lazy_import_deps_of_with_files(
        roots,
        root,
        tsconfig,
        max_depth,
        &graph_files,
    ))
}

pub(crate) fn lazy_import_deps_of_with_files(
    roots: &[NodeId],
    _root: &Path,
    tsconfig: &TsConfig,
    max_depth: Option<usize>,
    graph_files: &GraphFiles,
) -> Vec<NodeEntry> {
    let ts_ex = ImportExtractor::for_typescript().expect("typescript import extractor builds");
    let tsx_ex = ImportExtractor::for_tsx().expect("tsx import extractor builds");
    let resolver = ImportResolver::new(tsconfig).with_visible(&graph_files.visible);

    let mut visited: HashSet<NodeId> = HashSet::new();
    let mut frontier: Vec<NodeId> = Vec::new();
    let mut result: Vec<NodeEntry> = Vec::new();
    let mut result_idx: HashMap<NodeId, usize> = HashMap::new();

    for root in roots {
        visited.insert(root.clone());
        frontier.push(root.clone());
    }

    let mut depth = 0;
    while !frontier.is_empty() {
        if let Some(max) = max_depth {
            if depth >= max {
                break;
            }
        }

        let mut expanded: Vec<(NodeId, Vec<(NodeId, EdgeKind)>)> = frontier
            .par_iter()
            .map(|node| {
                let Some(path) = node.as_file() else {
                    return (node.clone(), Vec::new());
                };
                if !graph_files.is_visible(path) || !is_indexable(path) {
                    return (node.clone(), Vec::new());
                }
                (
                    node.clone(),
                    import_neighbors(path, &resolver, &ts_ex, &tsx_ex, graph_files),
                )
            })
            .collect();
        expanded.sort_by_key(|(node, _)| node_sort_key(node));

        let next_depth = depth + 1;
        let mut next_frontier = Vec::new();
        for (_node, neighbors) in expanded {
            for (neighbor, kind) in neighbors {
                if visited.insert(neighbor.clone()) {
                    let idx = result.len();
                    result.push(NodeEntry {
                        node: neighbor.clone(),
                        depth: next_depth,
                        via: vec![kind],
                    });
                    result_idx.insert(neighbor.clone(), idx);
                    next_frontier.push(neighbor);
                } else {
                    if let Some(&idx) = result_idx.get(&neighbor) {
                        add_via_kind(&mut result[idx], kind);
                    }
                }
            }
        }
        frontier = next_frontier;
        depth = next_depth;
    }

    result
}

fn import_neighbors(
    path: &Path,
    resolver: &ImportResolver<'_>,
    ts_ex: &ImportExtractor,
    tsx_ex: &ImportExtractor,
    graph_files: &GraphFiles,
) -> Vec<(NodeId, EdgeKind)> {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(_) => return Vec::new(),
    };
    let extractor = if is_tsx_file(path) { tsx_ex } else { ts_ex };
    let mut neighbors: Vec<(NodeId, EdgeKind)> = extractor
        .extract(&source)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|imp| {
            resolver
                .resolve(&imp.specifier, path)
                .filter(|target| graph_files.is_visible(target))
                .map(|target| (NodeId::File(target), edge_kind_for_import(&imp)))
        })
        .collect();
    neighbors.sort_by_key(|(node, kind)| (node_sort_key(node), *kind as u8));
    neighbors
}

fn push_route_ref_edge(edges: &mut Vec<Edge>, source: &Path, target: &Path) {
    edges.push((
        NodeId::File(source.to_path_buf()),
        NodeId::File(target.to_path_buf()),
        EdgeKind::RouteRef,
    ));
}

fn add_distinct_worker_file_edges(
    forward: &mut EdgeMap,
    reverse: &mut EdgeMap,
    worker_file: &PathBuf,
    processor_file: &PathBuf,
    queue_job: &NodeId,
) {
    if worker_file != processor_file {
        add_edge(
            forward,
            queue_job.clone(),
            NodeId::File(worker_file.clone()),
            EdgeKind::QueueWorker,
        );
        add_edge(
            reverse,
            NodeId::File(worker_file.clone()),
            queue_job.clone(),
            EdgeKind::QueueWorker,
        );
    }
}

fn bfs(
    starts: &[NodeId],
    edges: &EdgeMap,
    max_depth: Option<usize>,
    allowed: Option<&HashSet<EdgeKind>>,
) -> Vec<NodeEntry> {
    let mut visited: HashSet<NodeId> = HashSet::new();
    let mut queue: VecDeque<(NodeId, usize)> = VecDeque::new();
    let mut result: Vec<NodeEntry> = Vec::new();
    let mut result_idx: HashMap<NodeId, usize> = HashMap::new();

    for s in starts {
        visited.insert(s.clone());
        queue.push_back((s.clone(), 0));
    }

    while let Some((node, depth)) = queue.pop_front() {
        if let Some(max) = max_depth {
            if depth >= max {
                continue;
            }
        }

        if let Some(neighbors) = edges.get(&node) {
            for (neighbor, kind) in neighbors {
                if let Some(allowed) = allowed {
                    if !allowed.contains(kind) {
                        continue;
                    }
                }

                if visited.insert(neighbor.clone()) {
                    let next_depth = depth + 1;
                    let idx = result.len();
                    result.push(NodeEntry {
                        node: neighbor.clone(),
                        depth: next_depth,
                        via: vec![*kind],
                    });
                    result_idx.insert(neighbor.clone(), idx);
                    queue.push_back((neighbor.clone(), next_depth));
                } else if let Some(&idx) = result_idx.get(neighbor) {
                    add_via_kind(&mut result[idx], *kind);
                }
            }
        }
    }

    result
}

fn add_via_kind(entry: &mut NodeEntry, kind: EdgeKind) {
    if !entry.via.contains(&kind) {
        entry.via.push(kind);
        entry.via.sort_by_key(|k| *k as u8);
    }
}

// ── Sort key for deterministic adjacency ordering ─────────────────────────────

fn node_sort_key(n: &NodeId) -> String {
    match n {
        NodeId::File(p) => p.to_string_lossy().into_owned(),
        NodeId::QueueJob { queue_file, job } => {
            format!("{}#{}", queue_file.to_string_lossy(), job)
        }
    }
}

// ── Edge producers ────────────────────────────────────────────────────────────

fn collect_parsed_imports(
    files: &[PathBuf],
    ts_ex: &ImportExtractor,
    tsx_ex: &ImportExtractor,
) -> ParsedImports {
    files
        .par_iter()
        .map(|path| {
            let source = std::fs::read_to_string(path).unwrap_or_default();
            let extractor = if is_tsx_file(path) { tsx_ex } else { ts_ex };
            let imports = extractor.extract(&source).unwrap_or_default();
            (path.clone(), imports)
        })
        .collect()
}

fn collect_parsed_imports_from_facts(files: &[PathBuf], facts: &TsFactMap) -> ParsedImports {
    files
        .par_iter()
        .filter_map(|path| {
            facts
                .get(path)
                .map(|file_facts| (path.clone(), file_facts.imports.clone()))
        })
        .collect()
}

fn collect_import_edges(
    parsed_imports: &ParsedImports,
    resolver: &ImportResolver<'_>,
    graph_files: &GraphFiles,
) -> Vec<Edge> {
    parsed_imports
        .par_iter()
        .flat_map_iter(|(path, imports)| {
            imports
                .iter()
                .filter_map(|imp| {
                    resolver.resolve(&imp.specifier, path).and_then(|target| {
                        if !graph_files.is_visible(&target) {
                            return None;
                        }
                        let kind = edge_kind_for_import(imp);
                        Some((NodeId::File(path.clone()), NodeId::File(target), kind))
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn collect_workspace_edges(
    parsed_imports: &ParsedImports,
    _resolver: &ImportResolver<'_>,
    workspace: &crate::codebase::workspaces::WorkspaceMap,
    graph_files: &GraphFiles,
) -> Vec<Edge> {
    if workspace.packages.is_empty() {
        return vec![];
    }

    parsed_imports
        .par_iter()
        .flat_map_iter(|(path, imports)| {
            imports
                .iter()
                .filter_map(|imp| {
                    let spec = &imp.specifier;
                    if spec.starts_with('.') {
                        return None;
                    }
                    workspace.resolve_specifier(spec).and_then(|entry| {
                        if !graph_files.is_visible(&entry) {
                            return None;
                        }
                        Some((
                            NodeId::File(path.clone()),
                            NodeId::File(entry),
                            EdgeKind::WorkspaceImport,
                        ))
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn edge_kind_for_import(import: &ExtractedImport) -> EdgeKind {
    match import.kind {
        ImportKind::Static => EdgeKind::Import,
        ImportKind::Type => EdgeKind::TypeImport,
        ImportKind::Dynamic => EdgeKind::DynamicImport,
        ImportKind::Require => EdgeKind::Require,
    }
}

fn collect_workspace_manifest_edges(
    all_files: &[PathBuf],
    workspace: &crate::codebase::workspaces::WorkspaceMap,
    graph_files: &GraphFiles,
) -> Vec<Edge> {
    if workspace.packages.is_empty() {
        return vec![];
    }

    all_files
        .par_iter()
        .flat_map_iter(|path| {
            let mut edges = Vec::new();
            if path.file_name().and_then(|name| name.to_str()) != Some("package.json") {
                return edges;
            }
            let Ok(source) = std::fs::read_to_string(path) else {
                return edges;
            };
            let Ok(package_json) = serde_json::from_str::<serde_json::Value>(&source) else {
                return edges;
            };
            for name in package_dependency_names(&package_json) {
                let entry = workspace
                    .packages
                    .iter()
                    .find(|package| package.name == name)
                    .and_then(|package| package.entry.as_ref());
                let Some(entry) = entry else {
                    continue;
                };
                if !graph_files.is_visible(entry) {
                    continue;
                }
                edges.push((
                    NodeId::File(path.clone()),
                    NodeId::File(entry.clone()),
                    EdgeKind::WorkspaceImport,
                ));
            }
            edges
        })
        .collect()
}

fn package_dependency_names(package_json: &serde_json::Value) -> Vec<String> {
    let mut names = Vec::new();
    for field in [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ] {
        let Some(deps) = package_json.get(field).and_then(|value| value.as_object()) else {
            continue;
        };
        for (name, version) in deps {
            if version.as_str().is_some() {
                names.push(name.clone());
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

#[cfg(test)]
fn package_name_from_spec(spec: &str) -> &str {
    if spec.starts_with('@') {
        // @scope/pkg[/subpath]
        let after_scope = spec.trim_start_matches('@');
        let slash_idx = after_scope.find('/').map(|i| i + 1);
        if let Some(idx) = slash_idx {
            let after_first_slash = &after_scope[idx..];
            let end = after_first_slash
                .find('/')
                .map(|i| idx + i + 1)
                .unwrap_or(spec.len());
            &spec[..end]
        } else {
            spec
        }
    } else {
        // pkg[/subpath]
        match spec.find('/') {
            Some(idx) => &spec[..idx],
            None => spec,
        }
    }
}

/// Collect `TestOf` edges between source files and their test counterparts.
fn collect_test_edges(files: &[PathBuf]) -> Vec<Edge> {
    let file_set: HashSet<&PathBuf> = files.iter().collect();

    let test_exts = ["mts", "ts", "tsx", "mjs", "js", "jsx"];
    let test_variants = ["test", "spec"];

    files
        .par_iter()
        .flat_map_iter(|path| {
            let mut edges = Vec::new();
            let stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => return edges,
            };
            let dir = path.parent().unwrap_or(Path::new(""));

            let source_stem = test_variants.iter().find_map(|&v| {
                let suffix = format!(".{v}");
                stem.strip_suffix(&suffix).map(str::to_string)
            });

            if let Some(src_stem) = source_stem {
                for ext in &test_exts {
                    let src_path = dir.join(format!("{src_stem}.{ext}"));
                    if file_set.contains(&src_path) {
                        edges.push((
                            NodeId::File(path.clone()),
                            NodeId::File(src_path),
                            EdgeKind::TestOf,
                        ));
                    }
                }
            } else {
                for variant in &test_variants {
                    for ext in &test_exts {
                        let test_path = dir.join(format!("{stem}.{variant}.{ext}"));
                        if file_set.contains(&test_path) {
                            edges.push((
                                NodeId::File(test_path),
                                NodeId::File(path.clone()),
                                EdgeKind::TestOf,
                            ));
                        }
                    }
                }
            }
            edges
        })
        .collect()
}

/// Collect `MarkdownLink` edges from `.md` files to the files they link to.
fn collect_md_edges(all_files: &[PathBuf], graph_files: &GraphFiles) -> Vec<Edge> {
    let md_files: Vec<PathBuf> = all_files
        .iter()
        .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("md" | "mdx")))
        .cloned()
        .collect();

    md_files
        .into_par_iter()
        .flat_map_iter(|path| {
            let source = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(_) => return vec![],
            };
            let dir = path.parent().unwrap_or(Path::new("")).to_path_buf();
            crate::codebase::md_links::extract_links(&source)
                .into_iter()
                .filter_map(|link| {
                    if crate::codebase::md_links::is_external(&link) {
                        return None;
                    }
                    let target = dir.join(&link);
                    let target_str = target.to_string_lossy();
                    let clean = target_str
                        .split('?')
                        .next()
                        .unwrap_or(&target_str)
                        .split('#')
                        .next()
                        .unwrap_or(&target_str);
                    let target = PathBuf::from(clean);
                    // Resolve `..` lexically (no filesystem access) so the path
                    // matches the normalized form used elsewhere in the graph.
                    let target = crate::codebase::ts_resolver::normalize_path(&target);
                    if graph_files.is_visible(&target) {
                        Some((
                            NodeId::File(path.clone()),
                            NodeId::File(target),
                            EdgeKind::MarkdownLink,
                        ))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Add `CiInvocation` edges from workflow YAML files to Rust binary source files.
fn add_ci_edges(root: &Path, all_files: &[PathBuf], forward: &mut EdgeMap, reverse: &mut EdgeMap) {
    let bins = collect_cargo_bins(root, all_files);
    if bins.is_empty() {
        return;
    }

    // Walk .github/workflows/*.yml
    let workflows_dir = root.join(".github").join("workflows");
    for path in all_files
        .iter()
        .filter(|path| workflows_dir.is_dir() && path.starts_with(&workflows_dir))
    {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "yml" && ext != "yaml" {
            continue;
        }

        let source = std::fs::read_to_string(path).unwrap_or_default();

        let invocations = match crate::codebase::ci_workflows::extract_invocations(&source) {
            Ok(inv) => inv,
            Err(_) => continue,
        };

        for inv in invocations {
            let cargo_target_files = inv
                .cargo_targets
                .iter()
                .filter_map(|target| bins.get_cargo_target(target));
            let direct_binary_files = inv
                .binaries
                .iter()
                .filter(|binary_name| {
                    !inv.cargo_targets
                        .iter()
                        .any(|target| target.binary == **binary_name)
                })
                .filter_map(|binary_name| bins.by_name.get(binary_name));
            for source_file in cargo_target_files.chain(direct_binary_files) {
                add_file_edge(
                    forward,
                    path.clone(),
                    source_file.clone(),
                    EdgeKind::CiInvocation,
                );
                add_file_edge(
                    reverse,
                    source_file.clone(),
                    path.clone(),
                    EdgeKind::CiInvocation,
                );
            }
        }
    }
}

#[derive(Default)]
struct CargoBinIndex {
    by_name: HashMap<String, PathBuf>,
    by_package_and_name: HashMap<(String, String), PathBuf>,
}

impl CargoBinIndex {
    fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }

    fn insert(&mut self, package: Option<&str>, name: String, source_file: PathBuf) {
        self.by_name
            .entry(name.clone())
            .or_insert_with(|| source_file.clone());
        if let Some(package) = package {
            self.by_package_and_name
                .insert((package.to_string(), name), source_file);
        }
    }

    fn get_cargo_target(
        &self,
        target: &crate::codebase::ci_workflows::CargoTarget,
    ) -> Option<&PathBuf> {
        target
            .package
            .as_ref()
            .and_then(|package| {
                self.by_package_and_name
                    .get(&(package.clone(), target.binary.clone()))
            })
            .or_else(|| self.by_name.get(&target.binary))
    }
}

fn collect_cargo_bins(root: &Path, all_files: &[PathBuf]) -> CargoBinIndex {
    let root_manifest = root.join("Cargo.toml");
    let root_toml = match std::fs::read_to_string(&root_manifest) {
        Ok(s) => s,
        Err(_) => return CargoBinIndex::default(),
    };

    let mut bins = CargoBinIndex::default();
    add_manifest_bins(&root_manifest, &root_toml, &mut bins);

    let members = match crate::codebase::ci_workflows::parse_cargo_workspace_members(&root_toml) {
        Ok(members) => members,
        Err(_) => return bins,
    };
    let excludes = crate::codebase::ci_workflows::parse_cargo_workspace_excludes(&root_toml)
        .unwrap_or_default();
    let member_set = cargo_member_globset(&members);
    let exclude_set = cargo_member_globset(&excludes);

    for (manifest, parent) in all_files
        .iter()
        .filter(|path| {
            path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml")
                && path != &&root_manifest
        })
        .filter_map(|manifest| manifest.parent().map(|parent| (manifest, parent)))
    {
        let Ok(rel_dir) = parent.strip_prefix(root) else {
            continue;
        };
        let is_member = member_set
            .as_ref()
            .map(|set| set.is_match(rel_dir))
            .unwrap_or(true);
        if !is_member
            || exclude_set
                .as_ref()
                .is_some_and(|set| set.is_match(rel_dir))
        {
            continue;
        }
        let Ok(cargo_toml) = std::fs::read_to_string(manifest) else {
            continue;
        };
        add_manifest_bins(manifest, &cargo_toml, &mut bins);
    }

    bins
}

fn cargo_member_globset(members: &[String]) -> Option<globset::GlobSet> {
    if members.is_empty() {
        return None;
    }
    let mut builder = globset::GlobSetBuilder::new();
    for member in members {
        let glob = globset::GlobBuilder::new(member)
            .literal_separator(true)
            .build()
            .ok()?;
        builder.add(glob);
    }
    builder.build().ok()
}

fn add_manifest_bins(manifest: &Path, cargo_toml: &str, bins: &mut CargoBinIndex) {
    let Ok(parsed_bins) = crate::codebase::ci_workflows::parse_cargo_bins(cargo_toml) else {
        return;
    };
    let package = crate::codebase::ci_workflows::parse_cargo_package_name(cargo_toml)
        .ok()
        .flatten();
    let Some(manifest_dir) = manifest.parent() else {
        return;
    };
    for (name, rel_path) in parsed_bins {
        if let Some(source_file) = resolve_cargo_bin_source(manifest_dir, &name, &rel_path) {
            bins.insert(package.as_deref(), name, source_file);
        }
    }
}

fn resolve_cargo_bin_source(manifest_dir: &Path, name: &str, rel_path: &str) -> Option<PathBuf> {
    let declared = manifest_dir.join(rel_path);
    if declared.exists() {
        return Some(declared);
    }

    let nested = manifest_dir
        .join("src")
        .join("bin")
        .join(name)
        .join("main.rs");
    if nested.exists() {
        return Some(nested);
    }

    None
}

/// Collect `RouteRef` edges from route-referencing files to route-definition files.
/// Only fires if `.guardrailsrc.yml` has route-consistency configuration.
fn collect_route_edges(
    root: &Path,
    tsconfig: &TsConfig,
    all_files: &[PathBuf],
    facts: Option<&TsFactMap>,
    config_options: Option<&GraphConfigOptions>,
) -> Vec<Edge> {
    use crate::codebase::ts_routes::{defs_frontend, matcher, refs};
    use globset::{GlobBuilder, GlobSetBuilder};

    let Some(config_options) = config_options else {
        return vec![];
    };
    let opts = &config_options.route;

    if (opts.backend_pattern.is_empty() || opts.backend_register_object.is_empty())
        && opts.frontend_root.is_empty()
    {
        return vec![];
    }

    let mut all_defs: Vec<(PathBuf, String)> = Vec::new();
    let backend_defs =
        if !opts.backend_pattern.is_empty() && !opts.backend_register_object.is_empty() {
            let glob = match GlobBuilder::new(&opts.backend_pattern)
                .literal_separator(false)
                .build()
            {
                Ok(g) => g,
                Err(_) => return vec![],
            };
            let mut gb = GlobSetBuilder::new();
            gb.add(glob);
            let gs = gb
                .build()
                .expect("globset with one validated backend route glob should build");
            collect_backend_routes_from_graph_inputs(
                root,
                all_files,
                &opts.backend_register_object,
                &gs,
                facts,
            )
        } else {
            Vec::new()
        };
    all_defs.extend(backend_defs);
    if !opts.frontend_root.is_empty() {
        let frontend_abs = root.join(&opts.frontend_root);
        all_defs.extend(defs_frontend::collect_frontend_routes_from_files(
            &frontend_abs,
            all_files,
        ));
    }
    if all_defs.is_empty() {
        return vec![];
    }

    let mut pattern_to_files: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for (file, pattern) in &all_defs {
        pattern_to_files
            .entry(pattern.clone())
            .or_default()
            .push(file.clone());
    }
    let all_patterns: Vec<String> = pattern_to_files.keys().cloned().collect();

    let backend_prefixes = opts.backend_prefixes.clone();
    let backend_exact = opts.backend_exact_paths.clone();

    let scan_globs: Vec<String> = if opts.scan_patterns.is_empty() {
        vec![
            "**/*.tsx".to_string(),
            "**/*.ts".to_string(),
            "**/*.mts".to_string(),
        ]
    } else {
        opts.scan_patterns.clone()
    };
    let mut scan_gb = GlobSetBuilder::new();
    for glob in scan_globs.iter().filter_map(|g| Glob::new(g).ok()) {
        scan_gb.add(glob);
    }
    let scan_gs = scan_gb
        .build()
        .expect("globset with individually validated scan globs should build");

    let scan_files: Vec<PathBuf> = all_files
        .iter()
        .filter(|p| {
            p.strip_prefix(root)
                .map(|rel| scan_gs.is_match(rel))
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    let _ = tsconfig;

    scan_files
        .into_par_iter()
        .flat_map_iter(|path| {
            let rel = path
                .strip_prefix(root)
                .expect("route scan files are rooted under the graph root")
                .to_path_buf();
            let rel_str = rel.to_string_lossy().into_owned();
            let route_refs = facts
                .and_then(|facts| facts.get(&path))
                .map(|file_facts| file_facts.route_refs.clone())
                .unwrap_or_else(|| {
                    let source = std::fs::read_to_string(&path).unwrap_or_default();
                    refs::extract_route_refs(&source, &rel_str)
                });
            let mut edges = Vec::new();
            for route_ref in route_refs {
                let is_backend = backend_prefixes
                    .iter()
                    .any(|p| route_ref.pattern.starts_with(p.as_str()));
                let is_backend = is_backend || backend_exact.contains(&route_ref.pattern);
                if !is_backend && opts.frontend_root.is_empty() {
                    continue;
                }
                for pattern in &all_patterns {
                    if matcher::matches(&route_ref.pattern, pattern) {
                        for def_file in pattern_to_files[pattern]
                            .iter()
                            .filter(|def_file| *def_file != &path)
                        {
                            push_route_ref_edge(&mut edges, &path, def_file);
                        }
                    }
                }
            }
            edges
        })
        .collect()
}

fn collect_backend_routes_from_graph_inputs(
    root: &Path,
    all_files: &[PathBuf],
    register_object: &str,
    pattern_globset: &GlobSet,
    facts: Option<&TsFactMap>,
) -> Vec<(PathBuf, String)> {
    if let Some(facts) = facts {
        return all_files
            .par_iter()
            .filter(|path| {
                path.strip_prefix(root)
                    .map(|rel| pattern_globset.is_match(rel))
                    .unwrap_or(false)
            })
            .filter_map(|path| facts.get(path).map(|file_facts| (path, file_facts)))
            .flat_map(|(path, file_facts)| {
                file_facts
                    .backend_routes
                    .iter()
                    .map(|(route, _line)| ((*path).clone(), route.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();
    }

    crate::codebase::ts_routes::defs_backend::collect_backend_routes_from_files(
        root,
        all_files,
        register_object,
        pattern_globset,
    )
}

/// Add `QueueEnqueue` and `QueueWorker` edges via virtual `QueueJob` nodes.
///
/// Per-job convention (GlideMQ / BullMQ):
///   Enqueue:  `<binding>.add('jobName', data)` or `.addBulk([{ name: 'jobName', ... }])`
///   Worker:   `new Worker('queueName', handler)` dispatching via
///             `import * as processors from './processors.mts'`
///
/// Only fires if `.guardrailsrc.yml` has `queue-dashboard-reachability` config.
fn add_queue_edges(
    root: &Path,
    resolver: &ImportResolver<'_>,
    files: &[PathBuf],
    facts: Option<&TsFactMap>,
    forward: &mut EdgeMap,
    reverse: &mut EdgeMap,
) {
    use crate::codebase::config::{load_config, QueueOptions};
    use crate::codebase::ts_queues::factory::{find_create_queue_line, find_queue_name};
    use crate::codebase::ts_queues::usage::extract_queue_usage;
    use globset::GlobBuilder;

    let config = match load_config(root) {
        Ok(c) => c,
        Err(_) => return,
    };

    let opts: QueueOptions = config.rule_options("queue-dashboard-reachability");

    if opts.queue_pattern.is_empty() || opts.factory_specifier.is_empty() {
        return;
    }

    let glob = match GlobBuilder::new(&opts.queue_pattern)
        .literal_separator(false)
        .build()
    {
        Ok(g) => g,
        Err(_) => return,
    };
    let mut gb = globset::GlobSetBuilder::new();
    gb.add(glob);
    let gs = gb
        .build()
        .expect("globset with one validated queue pattern should build");

    // Phase 1: Find queue-def files and their queue names.
    // queue_name → def_file  (only queues with string-literal names)
    let mut queue_name_to_def: HashMap<String, PathBuf> = HashMap::new();
    // def_file → queue_name (for reverse lookup)
    let mut def_to_queue_name: HashMap<PathBuf, String> = HashMap::new();

    for path in files {
        let rel = path
            .strip_prefix(root)
            .expect("queue files are rooted under the graph root");
        if !gs.is_match(rel) {
            continue;
        }
        let (create_line, queue_name) = facts
            .and_then(|facts| facts.get(path))
            .map(|file_facts| (file_facts.queue_create_line, file_facts.queue_name.clone()))
            .unwrap_or_else(|| {
                let source = std::fs::read_to_string(path).unwrap_or_default();
                (
                    find_create_queue_line(
                        &source,
                        &opts.factory_specifier,
                        &opts.factory_function,
                    ),
                    find_queue_name(&source, &opts.factory_specifier, &opts.factory_function),
                )
            });
        if create_line.is_none() {
            continue;
        }
        if let Some(queue_name) = queue_name {
            queue_name_to_def.insert(queue_name.clone(), path.clone());
            def_to_queue_name.insert(path.clone(), queue_name);
        }
    }

    if queue_name_to_def.is_empty() {
        return;
    }

    // Phase 2: For each file, extract queue usage. Collect:
    //   - EnqueueSites: (queue_def_file, job_name) per source file
    //   - WorkerSites: (queue_def_file, processor_file, job_names) per source file

    // enqueue_sites: (source_file, queue_def_file, job_name)
    let mut enqueue_sites: Vec<(PathBuf, PathBuf, String)> = Vec::new();
    // worker_sites: (worker_file, queue_def_file, processor_file, job_names)
    let mut worker_sites: Vec<(PathBuf, PathBuf, PathBuf, Vec<String>)> = Vec::new();
    let mut processor_job_names: HashMap<PathBuf, Vec<String>> = HashMap::new();

    let queue_def_paths: HashSet<PathBuf> = def_to_queue_name.keys().cloned().collect();

    for path in files {
        let usage = facts
            .and_then(|facts| facts.get(path))
            .and_then(|file_facts| file_facts.queue_usage.clone())
            .unwrap_or_else(|| {
                let source = std::fs::read_to_string(path).unwrap_or_default();
                extract_queue_usage(&source)
            });

        // Resolve which imports come from queue-def files.
        // Build: local_binding → queue_def_file
        let mut binding_to_queue_def: HashMap<String, PathBuf> = HashMap::new();
        for (local_binding, import_spec) in &usage.imports {
            if let Some(resolved) = resolver.resolve(import_spec, path) {
                if queue_def_paths.contains(&resolved) {
                    binding_to_queue_def.insert(local_binding.clone(), resolved);
                }
            }
        }

        // Enqueue sites.
        for call in &usage.enqueue_calls {
            if let (Some(queue_def), Some(job)) =
                (binding_to_queue_def.get(&call.binding), &call.job)
            {
                enqueue_sites.push((path.clone(), queue_def.clone(), job.clone()));
            }
        }

        // Worker registrations.
        for worker in &usage.worker_declarations {
            let queue_def = worker
                .queue_name
                .as_ref()
                .and_then(|name| queue_name_to_def.get(name))
                .cloned();
            let processors_file = worker
                .processors_specifier
                .as_ref()
                .and_then(|spec| resolver.resolve(spec, path));
            let (Some(queue_def), Some(processors_file)) = (queue_def, processors_file) else {
                continue;
            };

            let job_names = if let Some(job_names) = processor_job_names.get(&processors_file) {
                job_names.clone()
            } else {
                let job_names =
                    extract_processor_job_names(&processors_file, facts).unwrap_or_default();
                processor_job_names.insert(processors_file.clone(), job_names.clone());
                job_names
            };

            if !job_names.is_empty() {
                worker_sites.push((path.clone(), queue_def, processors_file, job_names));
            }
        }
    }

    // Phase 3: Build QueueJob nodes for matched (queue, job) pairs.
    // A job is "matched" if it appears in BOTH an enqueue site AND a worker.
    // Build index: (queue_def, job) → [enqueue_files]
    let mut enqueue_index: HashMap<(PathBuf, String), Vec<PathBuf>> = HashMap::new();
    for (src, queue_def, job) in &enqueue_sites {
        enqueue_index
            .entry((queue_def.clone(), job.clone()))
            .or_default()
            .push(src.clone());
    }

    for (worker_file, queue_def, processor_file, job_names) in &worker_sites {
        for job in job_names {
            let key = (queue_def.clone(), job.clone());
            let Some(enqueue_files) = enqueue_index.get(&key) else {
                continue;
            };

            let queue_job = NodeId::QueueJob {
                queue_file: queue_def.clone(),
                job: job.clone(),
            };

            // Ensure the QueueJob node exists in the forward map.
            forward.entry(queue_job.clone()).or_default();
            reverse.entry(queue_job.clone()).or_default();

            // Enqueue site → QueueJob.
            for enqueue_file in enqueue_files {
                add_edge(
                    forward,
                    NodeId::File(enqueue_file.clone()),
                    queue_job.clone(),
                    EdgeKind::QueueEnqueue,
                );
                add_edge(
                    reverse,
                    queue_job.clone(),
                    NodeId::File(enqueue_file.clone()),
                    EdgeKind::QueueEnqueue,
                );
            }

            // QueueJob → processor file.
            add_edge(
                forward,
                queue_job.clone(),
                NodeId::File(processor_file.clone()),
                EdgeKind::QueueWorker,
            );
            add_distinct_worker_file_edges(
                forward,
                reverse,
                worker_file,
                processor_file,
                &queue_job,
            );
            add_edge(
                reverse,
                NodeId::File(processor_file.clone()),
                queue_job.clone(),
                EdgeKind::QueueWorker,
            );
        }
    }
}

fn extract_processor_job_names(
    processors_file: &Path,
    facts: Option<&TsFactMap>,
) -> Option<Vec<String>> {
    use crate::codebase::ts_symbols::extract_symbols;

    if let Some(symbols) = facts
        .and_then(|facts| facts.get(processors_file))
        .and_then(|file_facts| file_facts.symbols.as_ref())
    {
        return Some(
            symbols
                .exports
                .iter()
                .filter(|e| is_processor_export_kind(&e.kind))
                .map(|e| e.name.clone())
                .collect(),
        );
    }

    let proc_source = std::fs::read_to_string(processors_file).unwrap_or_default();
    let is_tsx = processors_file
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e == "tsx" || e == "jsx")
        .unwrap_or(false);
    let symbols = extract_symbols(&proc_source, is_tsx).ok()?;
    Some(
        symbols
            .exports
            .into_iter()
            .filter(|e| is_processor_export_kind(&e.kind))
            .map(|e| e.name)
            .collect(),
    )
}

fn is_processor_export_kind(kind: &ExportKind) -> bool {
    matches!(
        kind,
        ExportKind::Function | ExportKind::Const | ExportKind::Let | ExportKind::Var
    )
}

/// Collect `RouteTest` edges from playwright test files to the frontend page files they visit.
/// Uses `route-consistency.frontendRoot` when configured, otherwise `web/app` when present.
fn collect_playwright_route_edges(root: &Path, all_files: &[PathBuf]) -> Vec<Edge> {
    let Ok(report) =
        crate::codebase::playwright_coverage::collect_report_from_files(root, None, &[], all_files)
    else {
        return vec![];
    };

    report
        .routes
        .into_iter()
        .flat_map(|route| {
            route.tests.into_iter().map(move |test| {
                (
                    NodeId::File(root.join(test.file)),
                    NodeId::File(root.join(route.file.clone())),
                    EdgeKind::RouteTest,
                )
            })
        })
        .collect()
}

// ── HTTP call edges ───────────────────────────────────────────────────────────

/// Collect `HttpCall` edges: files that make literal HTTP calls to paths that
/// match a backend route definition.
///
/// Route definitions and backend prefixes must be configured by
/// `http-route-static-paths`, `http-call-static-paths`, or legacy
/// `route-consistency` options.
/// HTTP client calls are any `.<verb>(literal_path)` or `fetch(literal_path)`
/// where `literal_path` starts with a known backend prefix.
///
/// Runs defensively: non-literal call sites produce no edge. The
/// `http-call-static-paths` guardrail enforces literal discipline.
fn collect_http_call_edges(
    root: &Path,
    tsconfig: &TsConfig,
    facts: Option<&TsFactMap>,
    files: &[(PathBuf, String)],
    graph_files: &[PathBuf],
    all_files: &[PathBuf],
    config_options: Option<&GraphConfigOptions>,
) -> Vec<Edge> {
    use crate::codebase::ts_http_calls::extract_http_calls;

    let Some(config_options) = config_options else {
        return vec![];
    };
    let Some(backend_pattern) = resolved_backend_pattern(config_options) else {
        return vec![];
    };
    let Some(register_object) = resolved_backend_register_object(config_options) else {
        return vec![];
    };
    let backend_prefixes = resolved_backend_prefixes(config_options);
    if backend_prefixes.is_empty() {
        return vec![];
    }

    let Some(gs) = compile_graph_glob(&backend_pattern) else {
        return vec![];
    };

    // Collect backend route definitions: (file, pattern)
    let route_defs =
        collect_backend_routes_from_graph_inputs(root, all_files, &register_object, &gs, facts);
    if route_defs.is_empty() {
        return vec![];
    }

    let prefix_strs: Vec<&str> = backend_prefixes.iter().map(String::as_str).collect();

    let _ = tsconfig; // reserved for future alias-aware call resolution

    if let Some(facts) = facts {
        return graph_files
            .par_iter()
            .filter_map(|caller| {
                facts
                    .get(caller)
                    .map(|file_facts| (caller.as_path(), file_facts.http_calls.as_slice()))
            })
            .flat_map_iter(|(caller, calls)| http_edges_for_calls(caller, calls, &route_defs))
            .collect();
    }

    // For each source file, find HTTP calls and match against route defs.
    files
        .par_iter()
        .flat_map_iter(|(caller, source)| {
            let calls = extract_http_calls(source, &prefix_strs);
            http_edges_for_calls(caller, &calls, &route_defs)
        })
        .collect()
}

fn http_edges_for_calls(
    caller: &Path,
    calls: &[crate::codebase::ts_http_calls::HttpCall],
    route_defs: &[(PathBuf, String)],
) -> Vec<Edge> {
    use crate::codebase::ts_routes::matcher;

    let mut edges = Vec::new();
    for call in calls {
        for (def_file, def_pattern) in route_defs {
            if def_file != caller && matcher::matches(&call.path, def_pattern) {
                edges.push((
                    NodeId::File(caller.to_path_buf()),
                    NodeId::File(def_file.clone()),
                    EdgeKind::HttpCall,
                ));
            }
        }
    }
    edges
}

// ── Process spawn edges ───────────────────────────────────────────────────────

/// Collect `ProcessSpawn` edges from any file that spawns another via
/// `spawn`/`exec`/`execFile`/`fork` or Playwright `webServer.command`.
///
/// String-literal and template-literal (quasis concatenated) commands are
/// resolved; dynamic expressions are silently skipped.
fn collect_process_spawn_edges(
    root: &Path,
    facts: Option<&TsFactMap>,
    files: &[(PathBuf, String)],
    graph_files: &[PathBuf],
) -> Vec<Edge> {
    use crate::codebase::ts_process_spawn::extract_spawn_edges;

    if let Some(facts) = facts {
        return graph_files
            .par_iter()
            .filter_map(|path| facts.get(path))
            .flat_map_iter(|file_facts| {
                file_facts.process_spawns.iter().map(|e| {
                    (
                        NodeId::File(e.spawner.clone()),
                        NodeId::File(e.entry.clone()),
                        EdgeKind::ProcessSpawn,
                    )
                })
            })
            .collect();
    }

    files
        .par_iter()
        .flat_map_iter(|(spawner, source)| {
            extract_spawn_edges(source, spawner, root)
                .into_iter()
                .map(|e| {
                    (
                        NodeId::File(e.spawner),
                        NodeId::File(e.entry),
                        EdgeKind::ProcessSpawn,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

// ── Filter spec ──────────────────────────────────────────────────────────────

/// A compiled filter specification handling both file-glob and folder-suffix patterns.
pub struct FilterSpec {
    file_set: Option<GlobSet>,
    folder_specs: Vec<FolderSpec>,
}

struct FolderSpec {
    ancestor_depth: usize,
    set: GlobSet,
}

/// Build a `FilterSpec` from patterns.
///
/// Patterns ending with `/` are folder patterns: they collapse matched files to
/// the folder ancestor at that depth.
///
/// Returns `None` if `patterns` is empty (no filter applied).
pub fn build_filter(patterns: &[String]) -> Result<Option<FilterSpec>> {
    if patterns.is_empty() {
        return Ok(None);
    }

    let mut file_builder = GlobSetBuilder::new();
    let mut has_file = false;
    let mut folder_specs: Vec<FolderSpec> = Vec::new();

    for pattern in patterns {
        if let Some(base) = pattern.strip_suffix('/') {
            let ancestor_depth = Path::new(base).components().count();
            let file_glob = format!("{base}/**");
            let mut builder = GlobSetBuilder::new();
            builder.add(Glob::new(&file_glob)?);
            folder_specs.push(FolderSpec {
                ancestor_depth,
                set: builder.build()?,
            });
        } else {
            file_builder.add(Glob::new(pattern)?);
            has_file = true;
        }
    }

    Ok(Some(FilterSpec {
        file_set: if has_file {
            Some(file_builder.build()?)
        } else {
            None
        },
        folder_specs,
    }))
}

/// Retain only entries matching `filter`.
/// QueueJob virtual nodes pass through without path-based filtering.
pub fn apply_filter(
    entries: Vec<NodeEntry>,
    filter: Option<&FilterSpec>,
    root: &Path,
) -> Vec<NodeEntry> {
    let filter = match filter {
        None => return entries,
        Some(f) => f,
    };

    let mut result: Vec<NodeEntry> = Vec::new();
    let mut folder_seen: HashMap<PathBuf, usize> = HashMap::new();

    for entry in entries {
        // Virtual nodes (QueueJob) pass through without file-path filtering.
        let file_path = match entry.node.as_file() {
            Some(p) => p,
            None => {
                result.push(entry);
                continue;
            }
        };

        let rel = file_path.strip_prefix(root).unwrap_or(file_path);

        let mut matched_folder = false;
        for spec in &filter.folder_specs {
            if spec.set.is_match(rel) {
                let folder: PathBuf = rel.components().take(spec.ancestor_depth).collect();
                if let Some(&idx) = folder_seen.get(&folder) {
                    if entry.depth < result[idx].depth {
                        result[idx].depth = entry.depth;
                    }
                } else {
                    let idx = result.len();
                    folder_seen.insert(folder.clone(), idx);
                    result.push(NodeEntry {
                        node: NodeId::File(root.join(&folder)),
                        depth: entry.depth,
                        via: entry.via.clone(),
                    });
                }
                matched_folder = true;
                break;
            }
        }
        if matched_folder {
            continue;
        }

        if let Some(gs) = &filter.file_set {
            if gs.is_match(rel) {
                result.push(entry);
            }
        }
    }

    result
}

// ── Symbol index ─────────────────────────────────────────────────────────────

/// An importer record: (importer_file, local_name_used, is_re_export).
pub type ImporterRecord = (PathBuf, String, bool);

/// Index mapping (source_file, exported_symbol) → list of files importing that symbol.
pub struct SymbolIndex {
    map: HashMap<(PathBuf, String), Vec<ImporterRecord>>,
}

impl SymbolIndex {
    pub fn build(symbols_by_file: &HashMap<PathBuf, Vec<(PathBuf, String, String, bool)>>) -> Self {
        let mut map: HashMap<(PathBuf, String), Vec<ImporterRecord>> = HashMap::new();

        for (importer, imports) in symbols_by_file {
            for (source, imported_name, local_name, is_reexport) in imports {
                map.entry((source.clone(), imported_name.clone()))
                    .or_default()
                    .push((importer.clone(), local_name.clone(), *is_reexport));
            }
        }

        Self { map }
    }

    /// Build a symbol import index for every indexable file under `root`.
    ///
    /// This is the companion index required by `DepGraph::dependents_of_symbol`
    /// for `file#exportName` queries.
    pub fn build_from_root(root: &Path, tsconfig: &TsConfig) -> Result<Self> {
        let graph_files = GraphFiles::discover(root);
        Ok(Self::build_from_files(tsconfig, &graph_files))
    }

    pub(crate) fn build_from_files(tsconfig: &TsConfig, graph_files: &GraphFiles) -> Self {
        let facts = collect_ts_facts(graph_files.indexable(), TsFactPlan::imports_and_symbols());
        Self::build_from_facts(tsconfig, graph_files, &facts)
    }

    pub(crate) fn build_from_facts(
        tsconfig: &TsConfig,
        graph_files: &GraphFiles,
        facts: &TsFactMap,
    ) -> Self {
        type SymEntry = (PathBuf, String, String, bool);

        let resolver = ImportResolver::new(tsconfig).with_visible(graph_files.visible());

        let per_file: Vec<(PathBuf, Vec<SymEntry>)> = graph_files
            .indexable()
            .par_iter()
            .filter_map(|path| {
                let symbols = facts.get(path)?.symbols.as_ref()?;

                let mut imports_for_file = Vec::new();
                for ni in &symbols.imports {
                    if let Some(target) = resolver.resolve(&ni.source, path) {
                        imports_for_file.push((
                            target,
                            ni.imported.clone(),
                            ni.local.clone(),
                            false,
                        ));
                    }
                }
                for exp in &symbols.exports {
                    if let crate::codebase::ts_symbols::ExportKind::ReExport { source, imported } =
                        &exp.kind
                    {
                        if let Some(target) = resolver.resolve(source, path) {
                            imports_for_file.push((
                                target,
                                imported.clone(),
                                exp.name.clone(),
                                true,
                            ));
                        }
                    }
                }

                if imports_for_file.is_empty() {
                    None
                } else {
                    Some((path.clone(), imports_for_file))
                }
            })
            .collect();

        let symbols_by_file: HashMap<PathBuf, Vec<SymEntry>> = per_file.into_iter().collect();

        Self::build(&symbols_by_file)
    }

    pub fn importers_of(&self, source: &Path, symbol: &str) -> Option<&Vec<ImporterRecord>> {
        self.map.get(&(source.to_path_buf(), symbol.to_string()))
    }
}

#[cfg(test)]
mod tests;
