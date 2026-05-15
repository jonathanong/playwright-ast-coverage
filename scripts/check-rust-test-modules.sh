#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v rg >/dev/null 2>&1; then
    echo "Error: rg (ripgrep) is required."
    echo "Install with: brew install ripgrep  (or apt-get install ripgrep)"
    exit 1
fi

# rg exits 0 on match, 1 on no match, 2+ on error (including missing PCRE2 support).
# Capture stdout and stderr separately; temporarily disable errexit to capture the exit code.
_rg_stderr_tmp=$(mktemp)
trap 'rm -f "$_rg_stderr_tmp"' EXIT
set +e
rg_stdout=$(rg -n -U --pcre2 \
    '#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\](?:\s*|//.*|/\*[\s\S]*?\*/|#\s*\[[^\]]*\])*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{' \
    --glob '**/src/**/*.rs' \
    crates 2>"$_rg_stderr_tmp")
rg_exit=$?
rg_stderr=$(cat "$_rg_stderr_tmp")
set -e

if [ "$rg_exit" -ge 2 ]; then
    echo "Error: rg failed (exit $rg_exit):" >&2
    echo "$rg_stderr" >&2
    exit 1
fi

if [ "$rg_exit" -eq 0 ]; then
    echo "Offending files:"
    echo "$rg_stdout"
    echo
    echo "Inline #[cfg(test)] mod ... { ... } blocks are not allowed."
    echo "Use an out-of-line test module instead:"
    echo
    echo "    #[cfg(test)]"
    echo "    mod tests;"
    echo
    echo "and put the tests in src/<module>/tests.rs"
    echo "or src/<module>/tests/mod.rs (with sibling files) for larger test modules"
    exit 1
fi

echo "No inline #[cfg(test)] mod blocks found."
