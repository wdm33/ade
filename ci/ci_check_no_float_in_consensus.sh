#!/usr/bin/env bash
set -euo pipefail

# CI: enforce that no f32/f64 token appears in ade_core::consensus.
# Strengthens T-CORE-02 + DC-CONS-{07,08,09} (S-B1).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$REPO_ROOT/crates/ade_core/src/consensus"

if [ ! -d "$TARGET" ]; then
    echo "FAIL: $TARGET does not exist."
    exit 1
fi

if grep -RnE '\bf(32|64)\b' "$TARGET" > /tmp/ade_float_hits 2>/dev/null; then
    echo "FAIL: floating-point token found in ade_core::consensus:"
    cat /tmp/ade_float_hits
    exit 1
fi

echo "OK: no f32/f64 in ade_core::consensus"
