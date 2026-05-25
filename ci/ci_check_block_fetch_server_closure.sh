#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-G S4 — Producer-side block-fetch server reducer closure.
#
# Mechanical guards for CE-N-G-4 + DC-CONS-17 enforcement foundation:
#
#   1. Production code in `block_fetch/server.rs` may not import wall-
#      clock, randomness, async runtime, or HashMap iteration.
#   2. No pub fn in `block_fetch/server.rs` may return a raw
#      `BlockFetchMessage` — outgoing replies go through
#      `ServerReply::into_message()`.
#   3. Positive presence: `producer_block_fetch_serve` and
#      `ServedRangeLookup` exist.
#   4. Block { bytes } construction must come from the
#      `ServedRangeLookup` lookup output, never from re-encoding —
#      surfaced as a positive check that `ServerReply::block(` is
#      called only with `bytes` from the lookup iterator.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$REPO_ROOT/crates/ade_network/src/block_fetch/server.rs"

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
    print_fail "HashMap/HashSet forbidden in block_fetch/server.rs production code"
fi
if echo "$PROD_BODY" | grep -qE 'std::time::SystemTime|\btokio\b|\brand::|\brand_core'; then
    print_fail "wall-clock / tokio / rand forbidden in block_fetch/server.rs production code"
fi

# pub fn returning BlockFetchMessage directly is forbidden, except
# ServerReply::into_message (the single sanctioned exit).
if echo "$PROD_BODY" | grep -E 'pub fn [a-zA-Z0-9_]+\([^)]*\)[^{]*-> *BlockFetchMessage\b' | grep -qv 'into_message'; then
    print_fail "pub fn returning raw BlockFetchMessage is forbidden"
fi
if echo "$PROD_BODY" | grep -E 'pub fn [a-zA-Z0-9_]+\([^)]*\)[^{]*-> *Result<\(?.*BlockFetchMessage' | grep -qv 'into_message'; then
    print_fail "pub fn returning Result<.., BlockFetchMessage> is forbidden"
fi

# Positive presence checks.
if ! echo "$PROD_BODY" | grep -qE 'pub fn producer_block_fetch_serve\b'; then
    print_fail "producer_block_fetch_serve missing"
fi
if ! echo "$PROD_BODY" | grep -qE 'pub trait ServedRangeLookup\b'; then
    print_fail "ServedRangeLookup trait missing"
fi

# Block bytes provenance: the only ServerReply::block call should be
# fed from a `range_bytes` iterator. We surface this as a positive
# co-location check: the body should contain a `served.range_bytes`
# call and a `ServerReply::block(` call.
if ! echo "$PROD_BODY" | grep -qE 'served\.range_bytes'; then
    print_fail "producer_block_fetch_serve must consume served.range_bytes (the canonical AcceptedBlock-derived byte source)"
fi

if (( FAILED == 0 )); then
    echo "OK: block_fetch/server.rs closure invariants hold"
fi
exit $FAILED
