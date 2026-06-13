#!/usr/bin/env bash
# ci_check_missing_bridge_fail_closed.sh -- PHASE4-N-AO S11 (DC-NODE-39).
#
# After a ForkChoiceWin adoption at tip X, a competing descendant whose parent
# chain cannot connect to the durable adopted tip / a durable stored ancestor
# within k must produce a STRUCTURED, observable, fence-holding MissingBridge
# outcome -- NEVER the pre-S11 silent no-op/stall. MissingBridge is a fail-closed
# outcome ONLY: never an adoption path, a rollback target, a candidate anchor, a
# reason to clear the fence, a reason to skip the missing parent, or a reason to
# admit the unreachable block. NO durable mutation on the MissingBridge path.
#
# Mechanical guards:
#   (A) the dispatch's walk-fail + materialize-fail paths emit a structured
#       MissingBridge AND set the pending_missing_bridge HOLD (no bare silent
#       `return Ok(())` on those two arms).
#   (B) "missing_bridge" is a closed discriminator in the allow-list + the closed
#       reason is the MissingBridgeReason enum (no free-form String).
#   (C) the forge-fence predicate references pending_missing_bridge (a MissingBridge
#       HOLDS the fence).
#   (D) MissingBridge is observe-only -- no authority path (BLUE walk / selector /
#       fence decision) reads the emitted event back.
set -euo pipefail

NL="crates/ade_node/src/node_lifecycle.rs"
FS="crates/ade_node/src/fork_switch.rs"
EV="crates/ade_node/src/admission_log/event.rs"
WR="crates/ade_node/src/admission_log/writer.rs"
CE="crates/ade_node/src/convergence_evidence.rs"
LCA="crates/ade_node/src/lca_walk.rs"
AGG="crates/ade_node/src/candidate_aggregator.rs"
fail() { echo "FAIL (ci_check_missing_bridge_fail_closed): $1" >&2; exit 1; }
for f in "$NL" "$FS" "$EV" "$WR" "$CE" "$LCA"; do [ -f "$f" ] || fail "module $f missing"; done

# (A) The dispatch's two pre-S11-silent paths now emit + HOLD. Isolate the
# dispatch_competing_fork_choice body and assert BOTH the walk-fail (map_lca_error)
# and the materialize-fail (LcaUnreachable) arms emit_missing_bridge AND set
# *pending_missing_bridge -- a structured fail-closed HOLD, not a silent no-op.
DISPATCH="$(awk '/fn dispatch_competing_fork_choice</{f=1} f{print} f&&/^}$/{exit}' "$NL")"
[ -n "$DISPATCH" ] || fail "dispatch_competing_fork_choice not found in $NL"
echo "$DISPATCH" | grep -Eq "map_lca_error\(" \
  || fail "the walk-fail path must map the LcaError to a closed MissingBridgeReason (map_lca_error)"
echo "$DISPATCH" | grep -Eq "MissingBridgeReason::LcaUnreachable" \
  || fail "the materialize-fail path must set MissingBridgeReason::LcaUnreachable"
# emit_missing_bridge appears for BOTH structured-fail arms (>= 2 emit calls).
EMITS="$(echo "$DISPATCH" | grep -Ec "emit_missing_bridge\(")"
[ "$EMITS" -ge 2 ] || fail "both the walk-fail and materialize-fail arms must emit_missing_bridge (found $EMITS)"
# Both arms set the HOLD (>= 2 assignments of *pending_missing_bridge = Some(...)).
HOLDS="$(echo "$DISPATCH" | grep -Ec "\*pending_missing_bridge = Some\(")"
[ "$HOLDS" -ge 2 ] || fail "both structured-fail arms must set *pending_missing_bridge = Some(reason) (found $HOLDS)"

# (B) "missing_bridge" is the closed discriminator + in the writer allow-list, and
# the reason is the closed MissingBridgeReason enum, never a free-form String.
grep -Eq "=> \"missing_bridge\"" "$EV" || fail "missing_bridge discriminator missing in $EV"
grep -Eq "\"missing_bridge\"" "$WR"    || fail "missing_bridge missing from the writer DISCRIMINATORS allow-list"
grep -Eq "pub enum MissingBridgeReason" "$FS" || fail "MissingBridgeReason closed enum missing in $FS"
# The MissingBridge event's reason field is &'static str fed by MissingBridgeReason::as_str
# (a closed discriminator), NOT a free-form owned String.
if awk '/MissingBridge \{/{f=1} f{print} f&&/\},?$/{exit}' "$EV" | grep -Eq "reason: String"; then
  fail "MissingBridge.reason must be the closed discriminator (&'static str), never a free-form String"
fi
grep -Eq "fn as_str\(&self\) -> &'static str" "$FS" \
  || fail "MissingBridgeReason must expose a closed as_str discriminator"

# (C) The forge-fence predicate HOLDS on an unresolved missing bridge: it references
# pending_missing_bridge (so a MissingBridge keeps the fence set).
FENCE_FN="$(awk '/pub fn fork_switch_fence_resolved\(/{f=1} f{print} f&&/^}$/{exit}' "$FS")"
[ -n "$FENCE_FN" ] || fail "fork_switch_fence_resolved not found in $FS"
echo "$FENCE_FN" | grep -Eq "pending_missing_bridge" \
  || fail "fork_switch_fence_resolved must reference pending_missing_bridge (a MissingBridge HOLDS the fence)"
echo "$FENCE_FN" | grep -Eq "pending_missing_bridge\.is_none\(\)" \
  || fail "the fence resolves only when pending_missing_bridge.is_none() (an unresolved bridge HOLDS it)"

# (D) Observe-only: the BLUE LCA walk + the candidate builder must NOT reference the
# missing-bridge event / emitter -- the tap lives in the RED dispatch only, and no
# authority path consumes it.
for f in "$LCA"; do
  if grep -Eq "emit_missing_bridge|MissingBridge|MissingBridgeReason" "$f"; then
    fail "observe-only violated: $f (BLUE walk) references the missing-bridge surface"
  fi
done
if [ -f "$AGG" ]; then
  if grep -Eq "emit_missing_bridge|MissingBridge" "$AGG"; then
    fail "observe-only violated: $AGG (candidate builder) references the missing-bridge surface"
  fi
fi
# The emit result must never gate control flow (fire-and-forget observe-only).
if grep -Eq "if .*emit_missing_bridge" "$NL"; then
  fail "a missing-bridge emit result must never gate control flow (observe-only)"
fi
# MissingBridge must NOT be an adoption / rollback path: the hold-set arms must not
# call commit_rollback / apply_chain_event / pump_block on the same arm. The two
# structured-fail arms `return Ok(())` immediately after setting the hold (no admit).
echo "$DISPATCH" | awk '
  /\*pending_missing_bridge = Some\(/ { hold=1 }
  hold && /(commit_rollback|apply_chain_event|pump_block)\(/ {
    print "ADMIT-AFTER-HOLD"; exit
  }
  hold && /return Ok\(\(\)\);/ { hold=0 }
' | grep -q "ADMIT-AFTER-HOLD" \
  && fail "MissingBridge must NOT admit / roll back the unreachable block (no durable mutation on the HOLD path)"

echo "OK: missing-bridge is structured + fence-holding + closed-reason + observe-only + no-durable-mutation (DC-NODE-39)"
