#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v rg >/dev/null 2>&1; then
    echo "Error: rg (ripgrep) is required."
    echo "Install with: brew install ripgrep  (or apt-get install ripgrep)"
    exit 1
fi

# rg exits 0 on match, 1 on no match, 2+ on error (including missing PCRE2 support).
# Temporarily disable errexit so we can capture the exit code.
set +e
rg -n -U --pcre2 \
    '#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\](?:\s*|//.*|/\*[\s\S]*?\*/|#\s*\[[^\]]*\])*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{' \
    --glob '*/src/**/*.rs' \
    crates
rg_exit=$?
set -e

if [ "$rg_exit" -ge 2 ]; then
    echo "Error: rg failed with exit code $rg_exit (PCRE2 may be unsupported or no crates found)" >&2
    exit 1
fi

if [ "$rg_exit" -eq 0 ]; then
    echo
    echo "Inline #[cfg(test)] mod ... { ... } blocks are not allowed."
    echo "Use an out-of-line test module instead:"
    echo
    echo "    #[cfg(test)]"
    echo "    mod tests;"
    echo
    echo "and put the tests in src/<module>/tests.rs"
    exit 1
fi

echo "No inline #[cfg(test)] mod blocks found."
