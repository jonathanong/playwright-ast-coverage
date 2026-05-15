#!/usr/bin/env bash
set -euo pipefail

if rg -n -U --pcre2 \
    '#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]\s*(?:\n\s*)*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{' \
    crates/*/src
then
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
