#!/usr/bin/env bash
set -euo pipefail

# Verify contract header present as first line of every .rs source file
# in BLUE crates only (ade_codec, ade_types, ade_crypto, ade_core).
# GREEN and RED crates are outside CE-04 scope.

BLUE_CRATES=("ade_codec" "ade_types" "ade_crypto" "ade_core")
EXPECTED_FIRST_LINE="// Core Contract:"

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

FAILED=0

for crate in "${BLUE_CRATES[@]}"; do
    SRC_DIR="$REPO_ROOT/crates/$crate/src"
    if [ ! -d "$SRC_DIR" ]; then
        echo "FAIL: Source directory not found: $SRC_DIR"
        FAILED=1
        continue
    fi

    while IFS= read -r -d '' file; do
        first_line=$(head -n 1 "$file")
        if [ "$first_line" != "$EXPECTED_FIRST_LINE" ]; then
            echo "FAIL: Missing contract header in $file"
            echo "  Expected: $EXPECTED_FIRST_LINE"
            echo "  Got:      $first_line"
            FAILED=1
        fi
    done < <(find "$SRC_DIR" -name '*.rs' -print0)
done

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: All BLUE crate source files have contract headers"
    exit 0
else
    exit 1
fi
