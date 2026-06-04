#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-G-M (CN-WIRE-11): real cardano-node ChainSync MsgFindIntersect compatibility.
#   (A) request decode accepts the points list as a CBOR indefinite-length array (9f..ff) -- SCOPED to
#       that list only; decode_array_header stays definite-only (no broadening).
#   (B) the serve reducer replies IntersectFound[Origin] for an Origin intersection.
# Pinned to captured real cardano-node fixtures (request + reply), never an Ade<->Ade round-trip.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PRIM="$REPO_ROOT/crates/ade_network/src/codec/primitives.rs"
CS="$REPO_ROOT/crates/ade_network/src/codec/chain_sync.rs"
SRV="$REPO_ROOT/crates/ade_network/src/chain_sync/server.rs"
REQ_FIX="$REPO_ROOT/corpus/network/n2n/chain_sync/c1privnet_follower_findintersect_recv.cbor"
REPLY_FIX="$REPO_ROOT/corpus/network/n2n/chain_sync/c1privnet_origin_intersect_recv.cbor"
ITEST="$REPO_ROOT/crates/ade_network/tests/serve_chainsync_findintersect_cardano_node_fixture.rs"
REG="$REPO_ROOT/docs/ade-invariant-registry.toml"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$PRIM" "$CS" "$SRV" "$REQ_FIX" "$REPLY_FIX" "$ITEST" "$REG"; do
    [[ -f "$f" ]] || print_fail "missing expected file $f"
done

# (A) scoped two-form array head helper exists ...
grep -Eq 'pub fn decode_array_head_two_form' "$PRIM" \
    || print_fail "(A) decode_array_head_two_form (scoped two-form array head) not found in primitives.rs"
grep -Eq 'pub fn try_consume_break' "$PRIM" \
    || print_fail "(A) try_consume_break not found in primitives.rs"
# ... and the GENERAL array decoder STILL rejects indefinite (no broadening).
grep -Eq 'indefinite-length array not allowed' "$PRIM" \
    || print_fail "(A) decode_array_header no longer rejects indefinite-length arrays -- the fix broadened the general decoder"

# (A) the indefinite points list is accepted for MsgFindIntersect ONLY (the scoped FindIntersect decoder).
grep -Eq 'fn decode_find_intersect_points' "$CS" \
    || print_fail "(A) decode_find_intersect_points (scoped FindIntersect points decoder) not found in chain_sync.rs"
grep -Eq 'decode_array_head_two_form' "$CS" \
    || print_fail "(A) chain_sync.rs FindIntersect decoder does not use the scoped two-form helper"
# the two-form helper must NOT be used outside its definition (primitives.rs) and the FindIntersect
# decoder (codec/chain_sync.rs): no broad indefinite-array support.
OTHER="$(grep -rl 'decode_array_head_two_form' "$REPO_ROOT/crates" --include=*.rs 2>/dev/null | grep -vE 'codec/primitives\.rs|codec/chain_sync\.rs' || true)"
[[ -z "$OTHER" ]] || print_fail "(A) decode_array_head_two_form used outside the scoped FindIntersect decoder: $OTHER"

# (B) the reducer answers IntersectFound[Origin] for an Origin intersection.
grep -Eq 'Point::Origin => Some\(Point::Origin\)' "$SRV" \
    || print_fail "(B) producer_chain_sync_serve FindIntersect arm does not resolve Origin -> IntersectFound[Origin]"

# request fixture is the exact captured real-node bytes (82 04 9f 80 80 ff).
REQ_HEX="$(od -An -tx1 "$REQ_FIX" 2>/dev/null | tr -d ' \n')"
[[ "$REQ_HEX" == "82049f8080ff" ]] \
    || print_fail "request fixture bytes != 82 04 9f 80 80 ff (got: $REQ_HEX)"

# pin tests exist (decode + reduce, from the captured real-node fixtures).
for t in real_cardano_node_findintersect_indefinite_points_list_decodes \
         real_cardano_node_findintersect_yields_intersect_found_origin; do
    grep -Eq "fn $t" "$ITEST" || print_fail "pin test $t missing from the integration fixture test"
done
grep -Eq 'fn producer_chain_sync_serve_find_intersect_origin_yields_intersect_found_origin' "$SRV" \
    || print_fail "reducer Origin pin test missing from server.rs"
grep -Eq 'fn decode_array_header_still_rejects_indefinite' "$PRIM" \
    || print_fail "scope-guard test decode_array_header_still_rejects_indefinite missing from primitives.rs"

# CN-WIRE-11 present and enforced in the registry.
awk '/id = "CN-WIRE-11"/{f=1} f&&/status = "enforced"/{print "ok"; exit}' "$REG" | grep -q ok \
    || print_fail "CN-WIRE-11 not present-and-enforced in the registry"

if [[ "$FAILED" -ne 0 ]]; then
    echo "ci_check_chainsync_findintersect_compat: FAILED"
    exit 1
fi
echo "ci_check_chainsync_findintersect_compat: OK (CN-WIRE-11 -- real cardano-node FindIntersect: scoped indefinite decode + Origin reply)"
