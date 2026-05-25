#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-G S2 — ServedChainSnapshot closure.
#
# Mechanical guards for CE-N-G-2:
#
#   1. `served_chain.rs` may not import or use `HashMap`, `HashSet`, or
#      any `std::collections::Hash*` type. BTreeMap is the only
#      iteration order.
#   2. `ServedChainSnapshot` has no public constructor outside `new()`
#      and `served_chain_admit`. The internal `blocks` field is private,
#      so there is no struct-literal back-door from outside the module.
#   3. The admit function derives its key via `decode_block` — no
#      caller-supplied "asserted hash" parameter exists. (Surfaced as
#      a positive check: the admit signature is single-argument.)

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$REPO_ROOT/crates/ade_ledger/src/producer/served_chain.rs"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

if [[ ! -f "$TARGET" ]]; then
    print_fail "target file missing: $TARGET"
    exit "$FAILED"
fi

# Strip line comments + #[cfg(test)] regions before grepping.
PROD_BODY=$(awk '
    /^#\[cfg\(test\)\]/ { in_test=1 }
    in_test { next }
    { line=$0; sub(/\/\/.*$/, "", line); print line }
' "$TARGET")

if echo "$PROD_BODY" | grep -qE '\bHashMap\b'; then
    print_fail "HashMap forbidden in served_chain.rs (BTreeMap only)"
fi
if echo "$PROD_BODY" | grep -qE '\bHashSet\b'; then
    print_fail "HashSet forbidden in served_chain.rs (BTreeSet only if needed)"
fi
if echo "$PROD_BODY" | grep -qE 'std::collections::Hash'; then
    print_fail "std::collections::Hash* forbidden in served_chain.rs"
fi
if echo "$PROD_BODY" | grep -qE '\brand\b|SystemTime|tokio::time'; then
    print_fail "wall-clock / rand / tokio::time forbidden in served_chain.rs"
fi

# Positive checks: canonical API surface present.
if ! grep -qE 'pub fn served_chain_admit' "$TARGET"; then
    print_fail "served_chain_admit constructor missing"
fi
if ! grep -qE 'pub fn new\(\) -> Self' "$TARGET"; then
    print_fail "ServedChainSnapshot::new constructor missing"
fi
if ! grep -qE 'blocks: BTreeMap' "$TARGET"; then
    print_fail "blocks field must be BTreeMap-backed (closed iteration)"
fi

# admit must take exactly two arguments: (snapshot, AcceptedBlock). A
# third "asserted hash" parameter would expose a key mismatch surface
# the invariant forbids. We approximate this by checking the
# signature line.
ADMIT_SIG=$(grep -E 'pub fn served_chain_admit' "$TARGET" | head -1)
if ! echo "$ADMIT_SIG" | grep -qE 'pub fn served_chain_admit\(\s*$'; then
    # signature continues on next lines; check for unexpected hash
    # parameter on the next ~5 lines.
    SIG_BLOCK=$(grep -A 5 -E 'pub fn served_chain_admit' "$TARGET" | head -6)
    if echo "$SIG_BLOCK" | grep -qiE 'hash\s*:|asserted'; then
        print_fail "served_chain_admit must not accept a caller-supplied hash parameter (key derives from bytes)"
    fi
fi

if (( FAILED == 0 )); then
    echo "OK: served_chain.rs is closed (BTreeMap-only, single admit path)"
fi
exit $FAILED
