#!/usr/bin/env bash
# ci_check_lca_anchor_walk.sh -- PHASE4-N-AO S7 (DC-NODE-38).
#
# Live multi-block fork-anchor discovery. A live competing branch is eligible for
# SELECT only when Ade walks its preserved parent links back to a DURABLE STORED
# fork anchor within k (BLOCK DEPTH, not slot distance), then validates the
# COMPLETE intermediate header chain from that anchor before selection. The cache
# is NOT authority: each entry self-binds (its map key == its own re-derived block
# hash) or the branch fails closed; the durable LCA is authority ONLY when ChainDb
# confirms slot AND hash; the walk is k-bounded by block depth (no slot subtraction).
set -euo pipefail

W="crates/ade_node/src/lca_walk.rs"
M="crates/ade_node/src/node_lifecycle.rs"
fail() { echo "FAIL (ci_check_lca_anchor_walk): $1" >&2; exit 1; }
[ -f "$W" ] || fail "module $W missing"
[ -f "$M" ] || fail "module $M missing"

# CODE = the walk module with standalone comment lines stripped (the doc comments
# legitimately name slots/peers/bounds when describing the discipline; checks are
# on CODE).
WCODE="$(grep -vE '^[[:space:]]*//' "$W")"

# (A) The walk stops ONLY at a durable ChainDb-stored block, and the anchor binds
# that STORED block's slot (slot+hash authority, DC-NODE-29) -- never a cached /
# peer-supplied header's slot.
echo "$WCODE" | grep -Eq 'get_block_by_hash\(&prev\)' \
  || fail "the LCA must be resolved against the durable store (get_block_by_hash)"
echo "$WCODE" | grep -Eq 'anchor_slot:[[:space:]]*stored\.slot' \
  || fail "the anchor must bind the durable STORED slot (stored.slot), never a cached header slot"
if echo "$WCODE" | grep -Eq 'anchor_slot:[[:space:]]*entry\.|anchor_slot:[[:space:]]*.*header\.slot'; then
  fail "the anchor slot must NOT come from a cached/peer header (only the durable stored block)"
fi

# (B) The k-bound is BLOCK DEPTH (a traversed-header counter), NOT slot distance.
echo "$WCODE" | grep -Eq 'depth[[:space:]]*>=[[:space:]]*k' \
  || fail "the walk must be k-bounded by traversed-header DEPTH (depth >= k)"
echo "$WCODE" | grep -Eq 'depth[[:space:]]*\+=[[:space:]]*1' \
  || fail "the walk must increment the block-depth counter per traversed header"
# No slot subtraction may drive the k check (empty slots must not affect eligibility).
if echo "$WCODE" | grep -Eq 'slot.*-.*slot|\.slot\.0[[:space:]]*-|saturating_sub.*slot'; then
  fail "the k-bound must be block depth -- NO slot subtraction may gate eligibility"
fi

# (C) Cache self-binding: the entry's stored block_hash must equal the map key it
# was looked up under, else CacheSelfBindingViolation (the cache is evidence, not a
# stringly map of peer claims).
echo "$WCODE" | grep -Eq 'block_hash[[:space:]]*!=[[:space:]]*cur_hash' \
  || fail "each visited cache entry must self-bind (block_hash != cur_hash -> fail closed)"
echo "$WCODE" | grep -Eq 'CacheSelfBindingViolation' \
  || fail "a self-binding violation must be a closed LcaError variant"

# (D) The closed LcaError failure surface -- every un-anchorable / unsafe branch
# fails closed; no Other/String escape hatch.
for v in NoDurableAncestorWithinK BranchGap ExceededK CacheSelfBindingViolation; do
  echo "$WCODE" | grep -Eq "$v" || fail "LcaError must include the closed variant $v"
done

# (E) The COMPLETE intermediate header chain is collected (every traversed header
# pushed; reversed to LCA+1..=tip order) -- no candidate with missing headers.
echo "$WCODE" | grep -Eq 'headers\.push\(' \
  || fail "the walk must collect every traversed header (headers.push)"
echo "$WCODE" | grep -Eq 'headers\.reverse\(\)' \
  || fail "the collected headers must be ordered LCA+1..=tip (headers.reverse)"

# (F) An absent intermediate (not durable, not cached) is a BranchGap -- the branch
# is incomplete and not selectable.
echo "$WCODE" | grep -Eq 'ok_or\(LcaError::BranchGap\)' \
  || fail "a missing intermediate header must fail closed as BranchGap"

# --- The dispatch wiring (node_lifecycle.rs): the FULL branch feeds S2. ---
# Region: decide_fork_switch .. run_participant_sync (covers dispatch_competing_fork_choice).
REGION="$(awk '/fn decide_fork_switch/{f=1} /pub async fn run_participant_sync/{f=0} f' "$M")"
[ -n "$REGION" ] || fail "could not locate the dispatch region in $M"
MCODE="$(echo "$REGION" | grep -vE '^[[:space:]]*//')"

# (G) The dispatch builds a MULTI-header candidate from the complete walked branch
# (&lca.headers), NOT a single immediate-parent candidate (std::slice::from_ref of
# the one received header).
echo "$MCODE" | grep -Eq 'walk_to_durable_lca\(' \
  || fail "the dispatch must discover the anchor via walk_to_durable_lca"
echo "$MCODE" | grep -Eq '&lca\.headers' \
  || fail "build_candidate_fragment must receive the COMPLETE branch (&lca.headers), not one header"
if echo "$MCODE" | grep -Eq 'from_ref\(&decoded\.header_input\)'; then
  fail "the dispatch must NOT feed a single immediate-parent header (pre-S7 1-deep-only path)"
fi

# (H) The k passed to the walk is the durable/config block-depth authority
# (security_param.0), never a peer-supplied value.
echo "$MCODE" | grep -Eq 'walk_to_durable_lca\([^)]*security_param\.0' \
  || echo "$MCODE" | grep -Eq 'security_param\.0,' \
  || fail "the walk's k must be the durable security_param.0 (block-depth authority)"

echo "OK: LCA walk is durable-slot+hash-anchored + block-depth-k-bounded + cache-self-bound + complete-headers; dispatch feeds the full multi-header branch (DC-NODE-38)"
