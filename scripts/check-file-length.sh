#!/usr/bin/env bash
# Enforce per-file line cap (clippy has no equivalent lint).
# Cap defaults to 400 lines; override via LIMIT env var.
set -euo pipefail

LIMIT="${LIMIT:-400}"
ROOT="${1:-crates}"
status=0

while IFS= read -r -d '' f; do
    lines=$(wc -l <"$f")
    if (( lines > LIMIT )); then
        printf 'FAIL %4d lines (>%d): %s\n' "$lines" "$LIMIT" "$f"
        status=1
    fi
done < <(find "$ROOT" -type f -name '*.rs' \
              -not -path '*/target/*' \
              -not -path '*/tests/fixtures/*' \
              -print0)

if [ $status -eq 0 ]; then
    echo "OK: all .rs files under $ROOT/ are <= $LIMIT lines"
fi
exit $status
