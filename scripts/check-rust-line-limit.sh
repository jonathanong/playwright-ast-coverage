#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v tokei >/dev/null 2>&1; then
    echo "Error: tokei is required."
    echo "Install with: brew install tokei  (or cargo install tokei)"
    exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
    echo "Error: jq is required."
    echo "Install with: brew install jq  (or apt-get install jq)"
    exit 1
fi

SRC_MAX=200
TEST_MAX=500
fail=0

# All workspace crates must live under crates/; this scope is intentional.
json=$(tokei crates --files --output json)

# Sanity check: ensure tokei produced at least one Rust report so schema changes are caught loudly.
rust_count=$(echo "$json" | jq -r '(.Rust?.reports // []) | length')
if [ "$rust_count" -eq 0 ]; then
    echo "Error: tokei reported 0 Rust files — either schema changed or crates/ is empty" >&2
    exit 1
fi

while IFS=$'\t' read -r lines file; do
    if [ -n "$file" ] && [[ "$lines" =~ ^[0-9]+$ ]] && [ "$lines" -gt "$SRC_MAX" ]; then
        echo "$file: $lines code lines exceeds max $SRC_MAX" >&2
        [ -n "${GITHUB_ACTIONS:-}" ] && echo "::error file=$file::$file has $lines code lines (max $SRC_MAX)"
        fail=1
    fi
done < <(echo "$json" | jq -r '.Rust?.reports[]?
  | select(.name | test("(^|/)crates/[^/]+/src/"))
  | select(.name | test("/tests[/.]") | not)
  | [.stats.code, .name] | @tsv')

while IFS=$'\t' read -r lines file; do
    if [ -n "$file" ] && [[ "$lines" =~ ^[0-9]+$ ]] && [ "$lines" -gt "$TEST_MAX" ]; then
        echo "$file: $lines code lines exceeds max $TEST_MAX" >&2
        [ -n "${GITHUB_ACTIONS:-}" ] && echo "::error file=$file::$file has $lines code lines (max $TEST_MAX)"
        fail=1
    fi
done < <(echo "$json" | jq -r '.Rust?.reports[]?
  | select(
      (.name | test("(^|/)crates/[^/]+/tests/")) or
      (.name | test("(^|/)crates/[^/]+/src/.+/tests[/.]")) or
      (.name | test("(^|/)crates/[^/]+/src/tests[/.]"))
    )
  | [.stats.code, .name] | @tsv')

if [ "$fail" -eq 0 ]; then
    echo "All Rust files within line limits (src ≤$SRC_MAX, tests ≤$TEST_MAX)."
fi

exit "$fail"
