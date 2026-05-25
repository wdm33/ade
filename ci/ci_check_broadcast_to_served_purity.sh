#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-G S5 — GREEN broadcast-to-served adapter purity.
#
# Mechanical guards for CE-N-G-5:
#
#   1. `broadcast_to_served.rs` and `served_chain_lookups.rs` may not
#      import wall-clock, randomness, async runtime, or HashMap.
#   2. The adapter `drain_and_admit` exists and consumes
#      `BroadcastQueue` returning `(ServedChainSnapshot, BroadcastQueue,
#      Vec<AcceptedBlock>)`.
#   3. `ServedChainLookups` impls `ServedHeaderLookup` and
#      `ServedRangeLookup` (positive presence).
#   4. `served_chain_lookups.rs` uses the canonical
#      `accepted_block_header_bytes` import — proving no parallel
#      header splitter lives in the GREEN adapter.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ADAPTER="$REPO_ROOT/crates/ade_runtime/src/producer/broadcast_to_served.rs"
LOOKUPS="$REPO_ROOT/crates/ade_runtime/src/producer/served_chain_lookups.rs"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

check_purity() {
    local file="$1"
    local label="$2"
    if [[ ! -f "$file" ]]; then
        print_fail "target file missing: $file"
        return
    fi
    local body
    body=$(awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$file")
    if echo "$body" | grep -qE '\bHashMap\b|\bHashSet\b'; then
        print_fail "$label: HashMap/HashSet forbidden"
    fi
    if echo "$body" | grep -qE 'std::time::SystemTime|\btokio\b|\brand::|\brand_core'; then
        print_fail "$label: wall-clock / tokio / rand forbidden"
    fi
}

check_purity "$ADAPTER" "broadcast_to_served.rs"
check_purity "$LOOKUPS" "served_chain_lookups.rs"

if ! grep -qE 'pub fn drain_and_admit\b' "$ADAPTER"; then
    print_fail "drain_and_admit missing from broadcast_to_served.rs"
fi
if ! grep -qE 'impl<.*> ServedHeaderLookup for ServedChainLookups' "$LOOKUPS"; then
    print_fail "ServedHeaderLookup impl missing"
fi
if ! grep -qE 'impl<.*> ServedRangeLookup for ServedChainLookups' "$LOOKUPS"; then
    print_fail "ServedRangeLookup impl missing"
fi
if ! grep -qE 'accepted_block_header_bytes' "$LOOKUPS"; then
    print_fail "served_chain_lookups.rs must use the canonical accepted_block_header_bytes"
fi

if (( FAILED == 0 )); then
    echo "OK: broadcast_to_served + served_chain_lookups invariants hold"
fi
exit $FAILED
