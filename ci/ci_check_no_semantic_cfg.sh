#!/usr/bin/env bash
set -euo pipefail

# Grep BLUE crate src/ for semantic cfg attributes (T-BUILD-01).
# Feature flags in authoritative code could alter semantics per build profile.

BLUE_CRATES=("ade_codec" "ade_types" "ade_crypto" "ade_core")

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

FAILED=0

for crate in "${BLUE_CRATES[@]}"; do
    SRC_DIR="$REPO_ROOT/crates/$crate/src"
    if [ ! -d "$SRC_DIR" ]; then
        continue
    fi

    matches=$(grep -rn '#\[cfg(feature' "$SRC_DIR" --include='*.rs' 2>/dev/null || true)
    if [ -n "$matches" ]; then
        echo "FAIL: Feature cfg found in BLUE crate $crate:"
        echo "$matches"
        FAILED=1
    fi

    matches=$(grep -rn 'cfg!(feature' "$SRC_DIR" --include='*.rs' 2>/dev/null || true)
    if [ -n "$matches" ]; then
        echo "FAIL: Feature cfg! found in BLUE crate $crate:"
        echo "$matches"
        FAILED=1
    fi
done

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: No semantic cfg attributes in BLUE crates"
    exit 0
else
    exit 1
fi
