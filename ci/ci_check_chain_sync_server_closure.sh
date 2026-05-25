#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-G S3 — Producer-side chain-sync server reducer closure.
#
# Mechanical guards for CE-N-G-3 (DC-PROTO-08):
#
#   1. Production code in `chain_sync/server.rs` may not import wall-
#      clock, randomness, async runtime, or HashMap iteration.
#   2. No pub fn in `chain_sync/server.rs` may return a raw
#      `ChainSyncMessage` — outgoing replies go through
#      `ServerReply::into_message()`.
#   3. Positive presence: `producer_chain_sync_serve` and
#      `producer_chain_sync_advance_tip` exist.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$REPO_ROOT/crates/ade_network/src/chain_sync/server.rs"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

if [[ ! -f "$TARGET" ]]; then
    print_fail "target file missing: $TARGET"
    exit "$FAILED"
fi

PROD_BODY=$(awk '
    /^#\[cfg\(test\)\]/ { in_test=1 }
    in_test { next }
    { line=$0; sub(/\/\/.*$/, "", line); print line }
' "$TARGET")

if echo "$PROD_BODY" | grep -qE '\bHashMap\b|\bHashSet\b'; then
    print_fail "HashMap/HashSet forbidden in chain_sync/server.rs production code (BTreeMap-only)"
fi
if echo "$PROD_BODY" | grep -qE 'std::time::SystemTime|\btokio\b|\brand::|\brand_core'; then
    print_fail "wall-clock / tokio / rand forbidden in chain_sync/server.rs production code"
fi

# pub fn returning ChainSyncMessage directly is forbidden — outgoing
# replies must go through ServerReply. `into_message` on ServerReply
# is the single sanctioned exit and is exempt.
if echo "$PROD_BODY" | grep -E 'pub fn [a-zA-Z0-9_]+\([^)]*\)[^{]*-> *ChainSyncMessage\b' | grep -qv 'into_message'; then
    print_fail "pub fn returning raw ChainSyncMessage is forbidden (outgoing replies must go through ServerReply::into_message)"
fi
if echo "$PROD_BODY" | grep -E 'pub fn [a-zA-Z0-9_]+\([^)]*\)[^{]*-> *Result<\(?.*ChainSyncMessage' | grep -qv 'into_message'; then
    print_fail "pub fn returning Result<.., ChainSyncMessage> is forbidden (outgoing replies must go through ServerReply::into_message)"
fi

# Positive presence checks.
if ! echo "$PROD_BODY" | grep -qE 'pub fn producer_chain_sync_serve\b'; then
    print_fail "producer_chain_sync_serve missing"
fi
if ! echo "$PROD_BODY" | grep -qE 'pub fn producer_chain_sync_advance_tip\b'; then
    print_fail "producer_chain_sync_advance_tip missing"
fi
if ! echo "$PROD_BODY" | grep -qE 'pub trait ServedHeaderLookup\b'; then
    print_fail "ServedHeaderLookup trait missing"
fi

if (( FAILED == 0 )); then
    echo "OK: chain_sync/server.rs closure invariants hold"
fi
exit $FAILED
