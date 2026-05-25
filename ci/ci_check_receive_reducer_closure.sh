#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-H S2 — receive_apply reducer closure.
#
# Mechanical guards for CE-N-H-2 (CN-CONS-08 + DC-CONS-19):
#
#   1. Production code in `receive/reducer.rs` may not import
#      wall-clock, randomness, async runtime, or HashMap.
#   2. The reducer must call admit_via_block_validity (the canonical
#      admission gate); positive grep.
#   3. The RollBackward arm must return RollbackOutOfScope; positive
#      grep.
#   4. The roll_forward helper must not assign to state.ledger or
#      state.chain_dep — invariant I-6 (RollForward never mutates
#      authoritative state).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$REPO_ROOT/crates/ade_ledger/src/receive/reducer.rs"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

if [[ ! -f "$TARGET" ]]; then
    print_fail "target file missing: $TARGET"
    exit "$FAILED"
fi

# Production body (strip #[cfg(test)] + comments).
PROD_BODY=$(awk '
    /^#\[cfg\(test\)\]/ { in_test=1 }
    in_test { next }
    { line=$0; sub(/\/\/.*$/, "", line); print line }
' "$TARGET")

if echo "$PROD_BODY" | grep -qE '\bHashMap\b|\bHashSet\b'; then
    print_fail "HashMap/HashSet forbidden in receive/reducer.rs production code"
fi
if echo "$PROD_BODY" | grep -qE 'std::time::SystemTime|\btokio\b|\brand::|\brand_core'; then
    print_fail "wall-clock / tokio / rand forbidden in receive/reducer.rs production code"
fi

# Positive grep: must call the canonical admission gate.
if ! echo "$PROD_BODY" | grep -qE 'admit_via_block_validity'; then
    print_fail "receive/reducer.rs must call admit_via_block_validity (single admission authority)"
fi

# Positive grep: RollBackward arm returns RollbackOutOfScope.
if ! echo "$PROD_BODY" | grep -qE 'RollbackOutOfScope'; then
    print_fail "receive/reducer.rs must surface ReceiveError::RollbackOutOfScope for RollBackward"
fi

# Negative grep on the roll_forward helper body: extract the
# function body and check it does not assign to state.ledger or
# state.chain_dep.
ROLL_FORWARD_BODY=$(awk '
    /fn roll_forward\(/ { in_fn=1; brace=0 }
    in_fn {
        # naive brace counting to find the function end
        for (i=1; i<=length($0); i++) {
            ch = substr($0, i, 1)
            if (ch == "{") brace++
            if (ch == "}") { brace--; if (brace == 0) { print; in_fn=0; next } }
        }
        if (in_fn) print
    }
' "$TARGET")
if echo "$ROLL_FORWARD_BODY" | grep -qE 'state\.ledger *=|state\.chain_dep *='; then
    print_fail "roll_forward helper must not mutate state.ledger or state.chain_dep (I-6)"
fi

if (( FAILED == 0 )); then
    echo "OK: receive/reducer.rs closure invariants hold"
fi
exit $FAILED
