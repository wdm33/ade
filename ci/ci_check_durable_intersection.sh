#!/usr/bin/env bash
set -euo pipefail

# SLICE LV-16 (DC-NODE-48 / T-REC-07) -- a recovered node finds its common ancestor with the peer.
#
# REFERENCE (ouroboros-consensus): ChainDB followerForward moves to the FIRST offered point on the server's
# chain and its next instruction is RollBack to it; ChainSync/Client.hs mkOffsets offers
# [0] ++ fib(n>=2) < l ++ [l], l = min k maxOffset; IntersectNotFound terminates that peer.
#
# Asserts:
#  (a) the pump's one FindIntersect carries the whole offer (offer.points()), never a hand-built list;
#  (b) IntersectFound and IntersectNotFound are read through the BLUE classifier (check_intersection_reply ->
#      classify_intersect_reply); the IntersectNotFound arm sends no RequestNext;
#  (c) neither intersection outcome reconnects (should_reconnect_after);
#  (d) the node builds every offer from the durable chain (durable_intersection_offer) at dial time; the
#      single-point start (wire_pump_start_point) is gone; the reader recomputes each block's hash
#      (HashMismatch) and offers a non-stored tip alone (without_ancestry);
#  (e) the BLUE selection uses Cardano's offsets and always offers the deepest point;
#  (f) the CE tests exist and ran.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

PUMP="crates/ade_runtime/src/admission/wire_pump.rs"
NODE="crates/ade_node/src/node_lifecycle.rs"
BLUE="crates/ade_ledger/src/rollback/intersection.rs"
for f in "$PUMP" "$NODE" "$BLUE"; do
    [[ -f "$f" ]] || { echo "FAIL (durable intersection): $f not found"; exit 1; }
done

FAILED=0
fail() { echo "FAIL (durable intersection): $1"; FAILED=1; }

# Production text: everything before the trailing test module, line comments stripped.
prod() { awk '/^#\[cfg\(test\)\]$/ { getline nxt; if (nxt ~ /^mod tests/) exit; print; print nxt; next } { print }' "$1" | sed -E 's://.*::'; }
isolate() { awk -v pat="$1" '$0 ~ pat { c=1 } c { print } c && /^}/ { exit }' <<<"$2"; }
between() { awk -v a="$1" -v b="$2" '$0 ~ a { c=1; print; next } c && $0 ~ b { exit } c { print }' <<<"$3"; }
n() { grep -cE "$1" <<<"$2" || true; }

PUMP_PROD="$(prod "$PUMP")"
NODE_PROD="$(prod "$NODE")"
BLUE_PROD="$(prod "$BLUE")"

# --- (a) one FindIntersect carrying the whole offer -------------------------------------------------
ENTRY="$(isolate '^pub async fn run_admission_wire_pump' "$PUMP_PROD")"
grep -qE 'points: offer\.points\(\)' <<<"$ENTRY" || fail "(a) the FindIntersect is not built from offer.points()"
(( $(n 'FindIntersect \{$' "$PUMP_PROD") == 1 )) || fail "(a) the pump builds more than one FindIntersect"
grep -qE 'points: vec!\[' <<<"$PUMP_PROD" && fail "(a) a FindIntersect point list is built by hand"

# --- (b) replies read through the BLUE classifier ---------------------------------------------------
HANDLE="$(isolate '^fn handle_chain_sync' "$PUMP_PROD")"
FOUND="$(between 'ChainSyncMessage::IntersectFound' 'ChainSyncMessage::IntersectNotFound' "$HANDLE")"
NOTFOUND="$(between 'ChainSyncMessage::IntersectNotFound' 'ChainSyncMessage::RollForward' "$HANDLE")"
grep -qE 'check_intersection_reply\(offer, Some\(&point\)' <<<"$FOUND" || fail "(b) IntersectFound is not checked against the offer"
grep -qE 'check_intersection_reply\(offer, None' <<<"$NOTFOUND" || fail "(b) IntersectNotFound is not read through the classifier"
grep -qE 'queue_chain_sync_request_next' <<<"$NOTFOUND" && fail "(b) IntersectNotFound still sends RequestNext"
CHECK="$(isolate '^fn check_intersection_reply' "$PUMP_PROD")"
grep -qE 'classify_intersect_reply\(' <<<"$CHECK" || fail "(b) check_intersection_reply does not call the BLUE classifier"

# --- (c) no reconnect after an intersection outcome --------------------------------------------------
POLICY="$(isolate '^fn should_reconnect_after' "$NODE_PROD")"
FALSE_ARM="$(between 'AdmissionWirePumpError::Session' '=> false' "$POLICY"; grep -E '=> false' <<<"$POLICY" | head -1)"
grep -qE 'IntersectionPointNotOffered' <<<"$FALSE_ARM" || fail "(c) IntersectionPointNotOffered is not in the no-reconnect arm"
grep -qE 'NoCommonIntersection' <<<"$FALSE_ARM" || fail "(c) NoCommonIntersection is not in the no-reconnect arm"

# --- (d) the node's offer comes from the durable chain ----------------------------------------------
grep -qE 'fn wire_pump_start_point' <<<"$NODE_PROD" && fail "(d) the single-point start (wire_pump_start_point) is back"
SPAWN="$(isolate '^fn spawn_live_wire_pump_source' "$NODE_PROD")"
grep -qE 'durable_intersection_offer\(' <<<"$SPAWN" || fail "(d) the supervisor does not build its offer from the durable chain"
grep -qE 'with_delivered_head\(' <<<"$SPAWN" || fail "(d) the reconnect does not put the delivered head first"
READ="$(isolate '^pub fn read_intersection_ancestry' "$NODE_PROD")"
grep -qE 'header\.hash != next' <<<"$READ" || fail "(d) the ancestry reader does not recompute the block hash"
grep -qE 'HashMismatch' <<<"$READ" || fail "(d) a hash mismatch is not refused"
OFFER="$(isolate '^pub fn durable_intersection_offer' "$NODE_PROD")"
grep -qE 'None => Ok\(IntersectOffer::Candidates\(IntersectionCandidates::without_ancestry\(' <<<"$OFFER" \
    || fail "(d) a non-stored tip is not offered alone"
(( $(n 'without_ancestry\(' "$OFFER") == 1 )) || fail "(d) the bootstrap anchor may be offered below stored blocks"

# --- (e) Cardano's offsets, deepest always offered ----------------------------------------------------
SELECT="$(isolate '^pub fn select_intersection_candidates' "$BLUE_PROD")"
grep -qE 'intersection_offsets\(k, retained\)' <<<"$SELECT" || fail "(e) the selection does not use intersection_offsets"
OFFS="$(isolate '^pub fn intersection_offsets' "$BLUE_PROD")"
grep -qE 'out\.push\(l\)' <<<"$OFFS" || fail "(e) the deepest offset l is not offered"

# --- (f) the CE tests exist and ran -----------------------------------------------------------------
BLUE_TESTS=(
    lv16_offsets_are_cardanos_mk_offsets
    lv16_selection_is_newest_first_and_always_offers_the_deepest
    lv16_killed_store_shape_offers_the_parent_and_finds_it
    lv16_ancestry_faults_are_typed
    lv16_reply_must_name_an_offered_point
    lv16_not_found_is_no_common_point_with_the_bound
    lv16_a_point_without_stored_ancestry_is_offered_alone
    lv16_delivered_head_goes_first_only_when_newer
)
PUMP_TESTS=(
    lv16_pump_offers_every_candidate_and_follows_the_found_parent
    lv16_pump_ends_on_intersect_not_found_without_request_next
    lv16_pump_refuses_an_unoffered_found_point
)
NODE_TESTS=(
    lv16_no_reconnect_after_an_intersection_outcome
)
INTEGRATION=(
    lv16_corpus_blocks_are_a_linked_chain
    lv16_offer_is_read_from_the_validated_durable_ancestry
    lv16_a_block_whose_hash_is_not_its_lookup_hash_is_refused
    lv16_a_tip_that_is_not_stored_is_offered_alone
    lv16_killed_store_shape_intersects_at_the_parent
    lv16_a_deeper_fork_intersects_at_the_newest_offered_common_point
    lv16_a_peer_sharing_no_offered_point_is_no_common_point
    lv16_rollback_to_the_found_parent_matches_a_run_that_never_saw_the_tip
)
OUT_B="$(cargo test -p ade_ledger --lib -- rollback::intersection 2>&1 || true)"
OUT_P="$(cargo test -p ade_runtime --lib -- admission::wire_pump 2>&1 || true)"
OUT_N="$(cargo test -p ade_node --lib -- lv16_ reconnect_policy 2>&1 || true)"
OUT_I="$(cargo test -p ade_node --test lv16_durable_intersection 2>&1 || true)"
for t in "${BLUE_TESTS[@]}"; do
    grep -qE "^test [a-z0-9_:]*::${t} \.\.\. ok$" <<<"$OUT_B" || fail "(f) $t did not run and pass"
done
for t in "${PUMP_TESTS[@]}"; do
    grep -qE "^test [a-z0-9_:]*::${t} \.\.\. ok$" <<<"$OUT_P" || fail "(f) $t did not run and pass"
done
for t in "${NODE_TESTS[@]}"; do
    grep -qE "^test [a-z0-9_:]*::${t} \.\.\. ok$" <<<"$OUT_N" || fail "(f) $t did not run and pass"
done
for t in "${INTEGRATION[@]}"; do
    grep -qE "^test ${t} \.\.\. ok$" <<<"$OUT_I" || fail "(f) $t did not run and pass"
done
for o in "$OUT_B" "$OUT_P" "$OUT_N" "$OUT_I"; do
    grep -qE 'test result: ok\.' <<<"$o" || { fail "(f) a test run did not pass"; break; }
    grep -qE 'test result: .*FAILED' <<<"$o" && { fail "(f) a test run failed"; break; }
done

if (( FAILED == 0 )); then
    echo "PASS: durable intersection (LV-16, DC-NODE-48 / T-REC-07)"
fi
exit "$FAILED"
