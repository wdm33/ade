#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-K — clock-seam closure (DC-NODE-03).
#
# Mechanical guards:
#
#   1. `crates/ade_runtime/src/clock.rs` is the SOLE site of
#      `SystemTime::now()` / `Instant::now()` within `ade_runtime`.
#      The orchestrator core, bootstrap, persistent writer, peer
#      sessions, leadership session, and server pump all consume
#      time via the `Clock` trait.
#   2. The orchestrator core files (`orchestrator/core.rs`,
#      `orchestrator/event.rs`, `orchestrator/state.rs`,
#      `orchestrator/mod.rs`) contain none of: `SystemTime`,
#      `Instant`, `tokio::time::*`, `tokio::spawn`, `rand::*`,
#      `HashMap`, `HashSet`, `f32`, `f64`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUNTIME_SRC="$REPO_ROOT/crates/ade_runtime/src"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

# Strip comments + cfg(test) blocks; one file's production body.
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

# --- Rule 1: SystemTime / Instant only in clock.rs ---
for f in $(find "$RUNTIME_SRC" -type f -name '*.rs'); do
    rel="${f#$REPO_ROOT/}"
    if [[ "$rel" == "crates/ade_runtime/src/clock.rs" ]]; then
        continue
    fi
    body=$(strip_for_grep "$f")
    if echo "$body" | grep -qE 'SystemTime::now|Instant::now|tokio::time::Instant'; then
        print_fail "wall-clock reachable in $rel — only crates/ade_runtime/src/clock.rs may read SystemTime/Instant"
    fi
done

# --- Rule 2: orchestrator core files are GREEN-pure ---
CORE_FILES=(
    "$RUNTIME_SRC/orchestrator/core.rs"
    "$RUNTIME_SRC/orchestrator/event.rs"
    "$RUNTIME_SRC/orchestrator/state.rs"
    "$RUNTIME_SRC/orchestrator/mod.rs"
)
for f in "${CORE_FILES[@]}"; do
    if [[ ! -f "$f" ]]; then
        # Slice S2 may not be merged yet; skip silently.
        continue
    fi
    body=$(strip_for_grep "$f")
    if echo "$body" | grep -qE 'SystemTime|\bInstant\b|tokio::time|tokio::spawn|\brand::|\brand_core'; then
        print_fail "orchestrator core $f contains forbidden RED constructs (clock/tokio/rand)"
    fi
    if echo "$body" | grep -qE '\bHashMap\b|\bHashSet\b'; then
        print_fail "orchestrator core $f contains HashMap/HashSet (use BTreeMap/BTreeSet)"
    fi
    if echo "$body" | grep -qE '\bf32\b|\bf64\b'; then
        print_fail "orchestrator core $f contains floating-point types"
    fi
done

if (( FAILED == 0 )); then
    echo "OK: clock seam invariants hold"
fi
exit $FAILED
