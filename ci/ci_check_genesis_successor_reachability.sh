#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-G-J S4 (DC-NODE-08): node-spine cold-start first-block reachability.
# The genesis-successor is reachable on the real --mode node spine when BOTH
# tips are None, but ONLY as block 0 + PrevHash::Genesis, under ONE cold-start
# convention, gated by recovered lineage + a forge-eligible feed, advancing no
# durable tip.
#
#   (a) forge_one_from_recovered consumes Option<&ChainTip> (the no-tip / genesis
#       case is representable).
#   (b) the cold-start (None) arm emits block 0 + PrevHash::Genesis via the single
#       forge_header_position authority.
#   (c) ONE cold-start convention: no `.unwrap_or(1)` magic default survives; the
#       Some-tip-without-height edge fails closed (ok_or RecoveredTipMissingBlockNo).
#   (d) the cold-start forge is gated (may_cold_start_forge) by recovered lineage
#       (seed_epoch_consensus_inputs.is_some()) + a forge-eligible feed
#       (FeedReason::is_forge_eligible).
#   (e) the forge engine advances no durable tip: forge_one_from_recovered takes
#       no ChainDb handle (it structurally cannot write the durable tip).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SYNC="$REPO_ROOT/crates/ade_node/src/node_sync.rs"
LIFE="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$SYNC" "$LIFE"; do
    [[ -f "$f" ]] || print_fail "missing expected source $f"
done
if (( FAILED != 0 )); then echo "FAIL: ci_check_genesis_successor_reachability"; exit 1; fi

# (a) Option<&ChainTip> tip.
grep -qE 'selected_tip: Option<&ChainTip>' "$SYNC" \
    || print_fail "forge_one_from_recovered must take selected_tip: Option<&ChainTip> (the genesis no-tip case)"

# (b) cold-start ⇒ block 0 + PrevHash::Genesis (single position authority).
grep -qE 'None => Ok\(\(0, PrevHash::Genesis\)\)' "$SYNC" \
    || print_fail "the cold-start (None) arm must emit (0, PrevHash::Genesis) in forge_header_position"

# (c) ONE cold-start convention: no `.unwrap_or(1)` magic default in code; the
#     Some-tip-without-height edge fails closed (the comments naming the pre-S4
#     behaviour lack the leading dot, so they do not match).
if grep -qE '\.unwrap_or\(1\)' "$SYNC"; then
    print_fail "the pre-S4 .unwrap_or(1) cold-start default must be gone from node_sync (one convention = 0)"
fi
grep -qE 'ok_or\(NodeForgeError::RecoveredTipMissingBlockNo\)' "$SYNC" \
    || print_fail "the Some-tip-without-recorded-height edge must fail closed (RecoveredTipMissingBlockNo), not default"

# (d) cold-start forge gated by recovered lineage + forge-eligible feed.
grep -qE 'fn may_cold_start_forge' "$LIFE" \
    || print_fail "the cold-start permission gate may_cold_start_forge must exist"
grep -qE 'seed_epoch_consensus_inputs\.is_some\(\)' "$LIFE" \
    || print_fail "the cold-start gate must require the recovered seed-epoch lineage (seed_epoch_consensus_inputs.is_some())"
grep -qE 'is_forge_eligible\(\)' "$LIFE" \
    || print_fail "the cold-start gate must require a forge-eligible feed (FeedReason::is_forge_eligible)"

# (e) the forge engine advances no durable tip: it takes no ChainDb handle.
if sed -n '/pub fn forge_one_from_recovered($/,/-> Result/p' "$SYNC" | grep -qE 'ChainDb|chaindb'; then
    print_fail "forge_one_from_recovered must not take a ChainDb handle (the genesis forge advances no durable tip)"
fi

if (( FAILED == 0 )); then
    echo "OK: DC-NODE-08 — node-spine cold-start reachability: forge_one_from_recovered(Option<&ChainTip>) emits block 0 + PrevHash::Genesis at cold start (one convention, no .unwrap_or(1)); gated by recovered lineage + forge-eligible feed (may_cold_start_forge); the forge engine takes no ChainDb handle (no durable tip advance)."
fi
exit $FAILED
