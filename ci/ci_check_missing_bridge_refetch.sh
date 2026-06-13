#!/usr/bin/env bash
# ci_check_missing_bridge_refetch.sh -- PHASE4-N-AO S14 (DC-NODE-41).
#
# The DC-NODE-39 floor is SAFE but PASSIVE: ChainSync streams each block once, so a
# winner-descendant whose bridge Ade missed can never be recovered by waiting. S14
# turns that safe halt into ACTIVE, validated catch-up: on a MissingBridge for a
# post-ForkChoiceWin WINNING-PEER descendant, Ade re-fetches the missing range and
# admits it in parent-link order -- OR remains fail-closed with a closed code. The
# floor remains the fallback. Mechanically:
#
#   (A) Closed evidence: range_refetch_started / range_refetch_completed are closed
#       discriminators (event.rs) + in the writer DISCRIMINATORS allow-list; the
#       completed outcome + the started reason are &'static str closed discriminators
#       (RangeRefetchOutcome / MissingBridgeReason), never free-form String.
#   (B) Winning-peer-only: the dispatch sets pending_range_refetch ONLY inside a guard
#       that matches the post_switch_follow winning peer (no loser / unknown-peer /
#       pre-switch fetch spam).
#   (C) pump_block is the SOLE admit: recover_missing_range admits ONLY via pump_block;
#       it NEVER commit_rollback / apply_chain_event the fetched bytes (recovery is a
#       forward LinearExtend admit, not a rollback/adopt -- S3/S4 own selection/adopt).
#   (D) Bounded retry: MAX_RANGE_REFETCH_ATTEMPTS + range_refetch_should_retry exist;
#       the relay-loop drive loops on range_refetch_should_retry (no unbounded spin).
#   (E) Fail-closed clear rule: the drive clears the missing-bridge HOLD ONLY on
#       is_admitted() (real admitted progress) -- a short / lying / unservable range
#       LEAVES the floor hold. The trigger sets pending_range_refetch ALONGSIDE the
#       floor hold (it layers on DC-NODE-39, never replaces it).
#   (F) Observe-only + not-a-selector: the re-fetch emit result never gates control
#       flow; the BLUE walk / candidate builder never reference the re-fetch surface;
#       the recovery path never calls select_best_chain / decide_fork_switch.
set -euo pipefail

NL="crates/ade_node/src/node_lifecycle.rs"
FS="crates/ade_node/src/fork_switch.rs"
EV="crates/ade_node/src/admission_log/event.rs"
WR="crates/ade_node/src/admission_log/writer.rs"
CE="crates/ade_node/src/convergence_evidence.rs"
LCA="crates/ade_node/src/lca_walk.rs"
AGG="crates/ade_node/src/candidate_aggregator.rs"
fail() { echo "FAIL (ci_check_missing_bridge_refetch): $1" >&2; exit 1; }
for f in "$NL" "$FS" "$EV" "$WR" "$CE" "$LCA"; do [ -f "$f" ] || fail "module $f missing"; done

# (A) Closed evidence vocabulary == allow-list (event discriminator + writer allow-list).
for e in range_refetch_started range_refetch_completed; do
  grep -Eq "=> \"$e\"" "$EV" || fail "missing discriminator for '$e' in $EV"
  grep -Eq "\"$e\""    "$WR" || fail "'$e' missing from the writer DISCRIMINATORS allow-list"
done
# The completed.outcome + started.reason are closed &'static str, NOT free-form String.
RRC="$(awk '/RangeRefetchCompleted \{/{f=1} f{print} f&&/\},?$/{exit}' "$EV")"
[ -n "$RRC" ] || fail "RangeRefetchCompleted variant not found in $EV"
grep -Eq "outcome: &'static str" <<< "$RRC" \
  || fail "RangeRefetchCompleted.outcome must be the closed discriminator (&'static str), never a String"
RRS="$(awk '/RangeRefetchStarted \{/{f=1} f{print} f&&/\},?$/{exit}' "$EV")"
[ -n "$RRS" ] || fail "RangeRefetchStarted variant not found in $EV"
grep -Eq "reason: &'static str" <<< "$RRS" \
  || fail "RangeRefetchStarted.reason must be the closed MissingBridgeReason discriminator (&'static str)"
# The recovery outcome is a CLOSED enum with a closed as_str (no free-form transcript).
grep -Eq "pub enum RangeRefetchOutcome" "$FS" || fail "RangeRefetchOutcome closed enum missing in $FS"
grep -Eq "fn as_str\(&self\) -> &'static str" "$FS" \
  || fail "RangeRefetchOutcome must expose a closed as_str discriminator"

# (B) Winning-peer-only trigger: the dispatch sets pending_range_refetch ONLY guarded
# by the post_switch_follow winning peer. Isolate dispatch_competing_fork_choice and
# assert the assignment is preceded (in the same arm) by a winning_peer == peer guard.
DISPATCH="$(awk '/fn dispatch_competing_fork_choice</{f=1} f{print} f&&/^}$/{exit}' "$NL")"
[ -n "$DISPATCH" ] || fail "dispatch_competing_fork_choice not found in $NL"
grep -Eq "\*pending_range_refetch = Some\(" <<< "$DISPATCH" \
  || fail "the dispatch must set *pending_range_refetch = Some(...) on an eligible winning-peer descendant"
# The assignment lives inside a winning_peer == peer guard (winning-peer-only).
awk '
  /winning_peer == peer/ { guard=1 }
  guard && /\*pending_range_refetch = Some\(/ { print "GUARDED"; exit }
' <<< "$DISPATCH" | grep -q "GUARDED" \
  || fail "pending_range_refetch must be set ONLY inside a winning_peer == peer guard (winning-peer-only; no fetch spam)"
# The eligibility also requires the descendant to be AHEAD of the durable tip.
grep -Eq "slot.0 > durable_tip.slot.0" <<< "$DISPATCH" \
  || fail "the trigger must require the descendant ahead of the durable tip (slot > durable_tip.slot)"

# (C) pump_block is the SOLE admit in recovery: recover_missing_range admits via
# pump_block and NEVER commit_rollback / apply_chain_event (no rollback/adopt of the
# fetched bytes -- it is a forward LinearExtend admit only).
RECOVER="$(awk '/pub fn recover_missing_range</{f=1} f{print} f&&/^}$/{exit}' "$NL")"
[ -n "$RECOVER" ] || fail "recover_missing_range not found in $NL"
grep -Eq "pump_block\(" <<< "$RECOVER" \
  || fail "recover_missing_range must admit via pump_block (the sole roll-forward admit)"
if grep -Eq "(commit_rollback|apply_chain_event)\(" <<< "$RECOVER"; then
  fail "recover_missing_range must NOT commit_rollback / apply_chain_event (recovery is a forward admit, never a rollback/adopt of fetched bytes)"
fi
# Only the Admitted outcome (target descendant reached) is forward progress.
grep -Eq "RangeRefetchOutcome::Admitted" <<< "$RECOVER" \
  || fail "recover_missing_range must return Admitted only when the target descendant is reached"

# (D) Bounded retry (RED policy): the cap + the predicate exist; the relay-loop drive
# loops on the predicate (no unbounded spin).
grep -Eq "MAX_RANGE_REFETCH_ATTEMPTS" "$FS" || fail "MAX_RANGE_REFETCH_ATTEMPTS bound missing in $FS"
grep -Eq "fn range_refetch_should_retry\(" "$FS" || fail "range_refetch_should_retry policy missing in $FS"
grep -Eq "while range_refetch_should_retry\(" "$NL" \
  || fail "the relay-loop drive must loop on range_refetch_should_retry (bounded retry, no spin)"

# (E) Fail-closed clear rule: isolate the S14 relay-loop drive block and assert the
# missing-bridge HOLD clears ONLY on is_admitted() (a short/lying/unservable range
# leaves the floor hold). Capture from the drive marker to the fence-resolution marker.
DRIVE="$(awk '/S14 \(DC-NODE-41\): consume an eligible range re-fetch/{f=1} f{print} f&&/S5 \(DC-NODE-28 resolution\): the forge fence clears/{exit}' "$NL")"
[ -n "$DRIVE" ] || fail "the S14 range-refetch relay-loop drive block not found in $NL"
grep -Eq "prefetch_branch_bodies\(" <<< "$DRIVE" \
  || fail "the drive must re-fetch via prefetch_branch_bodies (the S6 byte-only BlockFetch)"
grep -Eq "recover_missing_range\(" <<< "$DRIVE" \
  || fail "the drive must admit the re-fetched range via recover_missing_range"
grep -Eq "emit_range_refetch_started\(" <<< "$DRIVE" \
  || fail "the drive must emit range_refetch_started"
grep -Eq "emit_range_refetch_completed\(" <<< "$DRIVE" \
  || fail "the drive must emit range_refetch_completed (with the closed outcome)"
# The pending_missing_bridge clear in the drive is guarded by is_admitted().
awk '
  /if outcome.is_admitted\(\)/ { guard=1 }
  guard && /pending_missing_bridge = None/ { print "GUARDED"; exit }
' <<< "$DRIVE" | grep -q "GUARDED" \
  || fail "the drive must clear pending_missing_bridge ONLY on outcome.is_admitted() (a non-admitted range LEAVES the floor hold)"

# (F) Observe-only + not-a-selector.
# F1: the re-fetch emit result never gates control flow (fire-and-forget observe-only).
if grep -Eq "if .*emit_range_refetch_(started|completed)" "$NL"; then
  fail "a range-refetch emit result must never gate control flow (observe-only)"
fi
# F2: the BLUE LCA walk + candidate builder never reference the re-fetch surface.
for f in "$LCA"; do
  if grep -Eq "RangeRefetch|range_refetch|recover_missing_range" "$f"; then
    fail "observe-only violated: $f (BLUE walk) references the range-refetch surface"
  fi
done
if [ -f "$AGG" ]; then
  if grep -Eq "RangeRefetch|range_refetch|recover_missing_range" "$AGG"; then
    fail "observe-only violated: $AGG (candidate builder) references the range-refetch surface"
  fi
fi
# F3: recovery is NOT a selector -- recover_missing_range never selects/decides.
if grep -Eq "(select_best_chain|decide_fork_switch)\(" <<< "$RECOVER"; then
  fail "recover_missing_range must NOT select / decide a branch (S3 owns selection; S14 is recovery only)"
fi

echo "OK: missing-bridge range re-fetch is winning-peer-only + pump_block-sole-admit + bounded + fail-closed-fallback + closed-vocab + observe-only (DC-NODE-41)"
