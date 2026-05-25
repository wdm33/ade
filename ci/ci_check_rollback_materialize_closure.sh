#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-I S2 — materialize_rolled_back_state closure (CN-STORE-07).
#
# Mechanical guards:
#
#   1. Production code in `rollback/materialize.rs` may not import
#      wall-clock, randomness, async runtime, or HashMap.
#   2. The only `pub fn` in the `rollback/*` module tree returning
#      `(LedgerState, PraosChainDepState)` is
#      `materialize_rolled_back_state` (single-authority discipline).
#   3. Positive grep: the driver calls `block_validity` (the same
#      authority N-H's receive admit branch uses).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$REPO_ROOT/crates/ade_ledger/src/rollback/materialize.rs"
ROLLBACK_DIR="$REPO_ROOT/crates/ade_ledger/src/rollback"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

if [[ ! -f "$TARGET" ]]; then
    print_fail "target file missing: $TARGET"
    exit "$FAILED"
fi

strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

PROD_BODY=$(strip_for_grep "$TARGET")

if echo "$PROD_BODY" | grep -qE '\bHashMap\b|\bHashSet\b'; then
    print_fail "HashMap/HashSet forbidden in rollback/materialize.rs"
fi
if echo "$PROD_BODY" | grep -qE 'std::time::SystemTime|\btokio\b|\brand::|\brand_core'; then
    print_fail "wall-clock / tokio / rand forbidden in rollback/materialize.rs"
fi

# CN-STORE-07: the only pub fn returning (LedgerState, PraosChainDepState)
# across the whole rollback module tree.
mapfile -t HITS < <(
    for f in "$ROLLBACK_DIR"/*.rs; do
        [[ -f "$f" ]] || continue
        body=$(strip_for_grep "$f")
        if echo "$body" | grep -qE 'pub fn [a-zA-Z0-9_]+\b[^{]*\(LedgerState[^{]*PraosChainDepState'; then
            if [[ "$(basename "$f")" != "materialize.rs" ]]; then
                echo "  $f"
            fi
        fi
    done
)
if (( ${#HITS[@]} > 0 )); then
    print_fail "rollback module tree must have exactly one pub fn returning (LedgerState, PraosChainDepState):"
    for h in "${HITS[@]}"; do
        echo "$h"
    done
fi

# Positive: block_validity call site.
if ! echo "$PROD_BODY" | grep -qE 'block_validity\b'; then
    print_fail "materialize_rolled_back_state must call block_validity (single admission authority CN-CONS-08)"
fi
if ! echo "$PROD_BODY" | grep -qE 'pub fn materialize_rolled_back_state'; then
    print_fail "materialize_rolled_back_state missing"
fi

if (( FAILED == 0 )); then
    echo "OK: rollback/materialize.rs closure invariants hold"
fi
exit $FAILED
