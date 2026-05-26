#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-K S1 — bootstrap_initial_state closure (CN-NODE-01).
#
# Mechanical guards:
#
#   1. Production code in `bootstrap.rs` may not import wall-clock,
#      randomness, async runtime, or HashMap.
#   2. Exactly one `pub fn` in `crates/ade_runtime/src/bootstrap.rs`
#      returns the initial state triple
#      `(LedgerState, PraosChainDepState, Option<ChainTip>)`.
#   3. Positive grep: the function calls
#      `materialize_rolled_back_state` (single materialize authority
#      CN-STORE-07) in the warm-start branch.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$REPO_ROOT/crates/ade_runtime/src/bootstrap.rs"

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
    print_fail "HashMap/HashSet forbidden in bootstrap.rs"
fi
if echo "$PROD_BODY" | grep -qE 'std::time::SystemTime|\btokio\b|\brand::|\brand_core'; then
    print_fail "wall-clock / tokio / rand forbidden in bootstrap.rs"
fi

# CN-NODE-01: exactly one `pub fn` in bootstrap.rs (production
# body, comments / tests stripped).
PUB_FN_COUNT=$(echo "$PROD_BODY" | grep -cE 'pub fn [a-zA-Z0-9_]+')
if (( PUB_FN_COUNT != 1 )); then
    print_fail "bootstrap.rs must have exactly one pub fn (found $PUB_FN_COUNT)"
fi

if ! echo "$PROD_BODY" | grep -qE 'pub fn bootstrap_initial_state'; then
    print_fail "bootstrap_initial_state missing"
fi

# Positive: the function's return shape mentions LedgerState +
# PraosChainDepState + ChainTip.
if ! echo "$PROD_BODY" | tr '\n' ' ' | grep -qE 'pub fn bootstrap_initial_state[^{]*Result<[^>]*LedgerState[^>]*PraosChainDepState[^>]*ChainTip'; then
    print_fail "bootstrap_initial_state must return Result<(LedgerState, PraosChainDepState, Option<ChainTip>), _>"
fi

# Positive: warm-start branch calls the materialize authority.
if ! echo "$PROD_BODY" | grep -qE 'materialize_rolled_back_state\b'; then
    print_fail "bootstrap.rs must call materialize_rolled_back_state (single materialize authority CN-STORE-07)"
fi

if (( FAILED == 0 )); then
    echo "OK: bootstrap.rs closure invariants hold"
fi
exit $FAILED
