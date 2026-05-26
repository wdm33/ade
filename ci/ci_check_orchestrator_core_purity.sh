#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-K S2 — orchestrator core purity.
#
# The GREEN orchestrator core files MUST NOT contain RED constructs.
# Files covered:
#   - crates/ade_runtime/src/orchestrator/core.rs
#   - crates/ade_runtime/src/orchestrator/event.rs
#   - crates/ade_runtime/src/orchestrator/state.rs
#   - crates/ade_runtime/src/orchestrator/mod.rs
#
# Forbidden in production body (cfg(test) blocks stripped):
#   - SystemTime, Instant, tokio::time::*, tokio::spawn
#   - rand::*, rand_core::*
#   - HashMap, HashSet
#   - f32, f64 literals or types

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CORE_DIR="$REPO_ROOT/crates/ade_runtime/src/orchestrator"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

CORE_FILES=(
    "$CORE_DIR/core.rs"
    "$CORE_DIR/event.rs"
    "$CORE_DIR/state.rs"
    "$CORE_DIR/mod.rs"
)

for f in "${CORE_FILES[@]}"; do
    if [[ ! -f "$f" ]]; then
        print_fail "missing required orchestrator core file: $f"
        continue
    fi
    body=$(strip_for_grep "$f")
    rel="${f#$REPO_ROOT/}"
    if echo "$body" | grep -qE 'SystemTime|\bInstant\b|tokio::time|tokio::spawn'; then
        print_fail "$rel contains tokio/time constructs"
    fi
    if echo "$body" | grep -qE '\brand::|\brand_core'; then
        print_fail "$rel contains rand"
    fi
    if echo "$body" | grep -qE '\bHashMap\b|\bHashSet\b'; then
        print_fail "$rel contains HashMap/HashSet (use BTreeMap/BTreeSet)"
    fi
    if echo "$body" | grep -qE '\bf32\b|\bf64\b'; then
        print_fail "$rel contains floating-point types"
    fi
done

if (( FAILED == 0 )); then
    echo "OK: orchestrator core purity holds"
fi
exit $FAILED
