# no-mistakes-scripts

Portable shell checks for macOS and Linux.

```bash
npm install --save-dev no-mistakes no-mistakes-scripts
npx no-mistakes rust-no-inline-tests crates/no-mistakes/src
npx no-mistakes rust-max-lines-per-file crates/no-mistakes/src crates/no-mistakes/tests
npx no-mistakes agents-md-max-size
```

The scripts are also available directly:

```bash
npx no-mistakes-rust-no-inline-tests crates/*/src
npx no-mistakes-rust-max-lines-per-file crates/*/src crates/*/tests
npx no-mistakes-agents-md-max-size
```

## Commands

### `no-mistakes-rust-no-inline-tests <src_dir> [<src_dir>...]`

Fails when a Rust source file contains an inline `#[cfg(test)] mod ... { ... }`
block. Use an out-of-line test module instead:

```rust
#[cfg(test)]
mod tests;
```

Requires `rg` with PCRE2 support.

### `no-mistakes-rust-max-lines-per-file [options] <src_dir> [<tests_dir>]`

Fails when Rust files exceed the configured code-line limits. Defaults are 200
code lines for source files and 500 code lines for test files.

Options:

- `--src-max <lines>`
- `--test-max <lines>`
- `--exclude <path>`

Requires `tokei` and `jq`.

`no-mistakes-rust-max-file-size` is an alias for this command.

### `no-mistakes-agents-md-max-size [options] [<max_lines>] [<max_chars>]`

Fails when `AGENTS.md` or `CLAUDE.md` files exceed the configured size limits.
Defaults are 200 lines and 12000 characters.

Options:

- `--root <path>`: limit the check to a directory.

When run inside a Git worktree, GitHub annotations use repository-relative paths.
