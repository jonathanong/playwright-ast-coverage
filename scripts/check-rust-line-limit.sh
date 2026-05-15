#!/usr/bin/env bash
set -euo pipefail

SRC_MAX=200
TEST_MAX=500
fail=0

json=$(tokei crates --files --output json)

while IFS=$'\t' read -r lines file; do
    if [ -n "$file" ] && [ "$lines" -gt "$SRC_MAX" ]; then
        echo "::error file=$file::$file has $lines code lines (max $SRC_MAX)"
        fail=1
    fi
done < <(echo "$json" | jq -r '.Rust.reports[]
  | select(.name | test("/crates/[^/]+/src/"))
  | [.stats.code, .name] | @tsv')

while IFS=$'\t' read -r lines file; do
    if [ -n "$file" ] && [ "$lines" -gt "$TEST_MAX" ]; then
        echo "::error file=$file::$file has $lines code lines (max $TEST_MAX)"
        fail=1
    fi
done < <(echo "$json" | jq -r '.Rust.reports[]
  | select(.name | test("/crates/[^/]+/tests/"))
  | [.stats.code, .name] | @tsv')

if [ "$fail" -eq 0 ]; then
    echo "All Rust files within line limits (src ≤$SRC_MAX, tests ≤$TEST_MAX)."
fi

exit $fail
