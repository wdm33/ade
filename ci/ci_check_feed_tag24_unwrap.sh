#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-G-O (CN-WIRE-12): feed/receive-side BlockFetch tag-24 unwrap before authoritative decode.
#   The wire pump strips the protocol tag-24 wrapper via the SINGLE ade_codec authority
#   (decompose_blockfetch_block = unwrap_tag24) ONCE at the MsgBlock receive boundary, emitting bare
#   [era, block]; fail-closed (BlockFetchDecode) on a non-tag-24 payload. Receive-side mirror of the
#   serve-side compose_blockfetch_block (CN-WIRE-08). NOT a new decoder, NOT a second unwrap.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PUMP="$REPO_ROOT/crates/ade_runtime/src/admission/wire_pump.rs"
BF="$REPO_ROOT/crates/ade_network/src/codec/block_fetch.rs"
FORGE_T="$REPO_ROOT/crates/ade_node/tests/forge_succeeds.rs"
LOOP_T="$REPO_ROOT/crates/ade_node/tests/node_spine_serve_loopback.rs"
REG="$REPO_ROOT/docs/ade-invariant-registry.toml"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$PUMP" "$BF" "$FORGE_T" "$LOOP_T" "$REG"; do
    [[ -f "$f" ]] || print_fail "missing expected file $f"
done

# The feed/receive path (wire pump Block arm) strips the tag-24 wrapper via the SINGLE authority ...
grep -Eq 'decompose_blockfetch_block' "$PUMP" \
    || print_fail "wire_pump.rs does not call decompose_blockfetch_block on the BlockFetch receive path"
# ... and fails closed (BlockFetchDecode) rather than forwarding a non-tag-24 payload.
grep -Eq 'BlockFetchDecode' "$PUMP" \
    || print_fail "wire_pump.rs Block arm does not fail closed with BlockFetchDecode"

# The unwrap authority is SINGLE: decompose_blockfetch_block is defined exactly once (block_fetch.rs),
# and it is the ade_codec::unwrap_tag24 inverse -- no second / hand-rolled unwrap.
DEF_COUNT="$(grep -rE 'fn decompose_blockfetch_block' "$REPO_ROOT/crates" --include=*.rs | wc -l | tr -d ' ')"
[[ "$DEF_COUNT" == "1" ]] \
    || print_fail "decompose_blockfetch_block must be defined exactly once (the single authority); found $DEF_COUNT"
grep -Eq 'unwrap_tag24' "$BF" \
    || print_fail "decompose_blockfetch_block is not the ade_codec::unwrap_tag24 inverse in block_fetch.rs"

# The serve-side wrap (the CN-WIRE-08 mirror) is unchanged and still present.
grep -Eq 'pub fn compose_blockfetch_block' "$BF" \
    || print_fail "serve-side compose_blockfetch_block (the CN-WIRE-08 mirror) missing from block_fetch.rs"

# pin tests exist.
grep -Eq 'fn feed_unwrap_decodes_genesis_successor_block_zero' "$FORGE_T" \
    || print_fail "genesis-successor feed-unwrap pin (captured shape -> unwrap -> block 0/Genesis) missing from forge_succeeds.rs"
for t in block_fetch_unwraps_tag24_emitting_bare_block block_fetch_fails_closed_on_non_tag24_payload; do
    grep -Eq "fn $t" "$PUMP" || print_fail "wire_pump pin test $t missing"
done
grep -Eq 'fn node_spine_serve_loopback_follower_fetches_self_accepted_block' "$LOOP_T" \
    || print_fail "node-spine serve loopback (end-to-end wire-pump unwrap) pin missing"
# the loopback now asserts BARE delivery -- the wire pump already unwrapped, so the test must NOT
# re-decompose (that pattern would mean the wire pump is still forwarding the wrapper).
grep -Eq 'decompose_blockfetch_block' "$LOOP_T" \
    && print_fail "loopback test still calls decompose_blockfetch_block -- the wire pump now delivers bare bytes; assert &fetched == &block_bytes directly"

# CN-WIRE-12 present and enforced in the registry.
awk '/id = "CN-WIRE-12"/{f=1} f&&/status = "enforced"/{print "ok"; exit}' "$REG" | grep -q ok \
    || print_fail "CN-WIRE-12 not present-and-enforced in the registry"

if [[ "$FAILED" -ne 0 ]]; then
    echo "ci_check_feed_tag24_unwrap: FAILED"
    exit 1
fi
echo "ci_check_feed_tag24_unwrap: OK (CN-WIRE-12 -- feed-side BlockFetch tag-24 unwrap before decode, single authority, fail-closed)"
