## Performance Guidelines

### Shared state in parallel loops

Avoid `Mutex<HashMap<K, V>>` for caches accessed from rayon `par_iter()`. The
lock serialises every lookup and insert across all threads, eliminating most
parallel speedup. Use `DashMap<K, V>` instead:

```rust
// Bad – contended lock dominates runtime at high thread counts
let cache: Mutex<HashMap<PathBuf, Arc<Vec<PathBuf>>>> = Mutex::new(HashMap::new());

// Good – lock-free concurrent map; use entry() to preserve "compute-once" semantics
let cache: DashMap<PathBuf, Arc<Vec<PathBuf>>> = DashMap::new();
let deps = if let Some(hit) = cache.get(&key) {
    hit.clone()
} else {
    let computed = Arc::new(expensive_compute(&key));
    cache.entry(key.clone()).or_insert(computed).clone()
};
```

### Hoist per-iteration I/O and parsing out of hot loops

Never read from disk, spawn processes, or parse files inside a loop that runs
once per test file (or per any other O(N) entity). Instead, compute the
invariant data once before the loop and pass it in:

```rust
// Bad – reads and parses config on every iteration
for file in test_files.par_iter() {
    let setup = config::setup_files_for_test(root, config, rel_path)?;
    // ...
}

// Good – compute once, reuse across all iterations
let setup_data = config::precompute_setup_data(root, config)?;
for file in test_files.par_iter() {
    let setup = config::setup_files_for_test_precomputed(&rel_path, &setup_data);
    // ...
}
```

Common violations to watch for:
- Calling `discover_files` (which runs `git ls-files`) per test file
- Reading and parsing config files per test file
- Building `GlobSet`/`Regex` per test file

### Guard expensive discovery behind an early return

`discover_files` runs `git ls-files` (two child processes). Only call it when
you actually need the file list. Guard with an early return for the empty-input
case:

```rust
fn expand_config_patterns(root: &Path, patterns: Vec<String>) -> Vec<ConfigFile> {
    if patterns.is_empty() {
        return Vec::new();  // avoid git ls-files when nothing to expand
    }
    let files = discover_files(root, &[]);
    // ...
}
```

### Pre-compute BFS traversals in parallel before the per-entity loop

When every parallel work item needs a BFS traversal of the same graph, run all
BFS traversals up front in a single `par_iter()` pass so the results are cached
before the work loop begins. This avoids redundant traversals and lets the
expensive computation scale linearly:

```rust
// Pre-populate cache for all test files before the per-test loop
test_files.par_iter().for_each(|file| {
    let deps = Arc::new(runtime_deps(&graph, file.clone()));
    dependency_cache.entry(file.clone()).or_insert(deps);
});

// Now every per-test reachable check is a cache hit
test_files.into_par_iter().map(|file| {
    reachable::check(/* ... uses dependency_cache ... */)?;
    // ...
})
```
